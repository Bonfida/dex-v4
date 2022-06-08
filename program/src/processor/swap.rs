use crate::{
    error::DexError,
    state::{CallBackInfo, DexState, FeeTier},
    utils::{check_account_key, check_account_owner, check_signer},
};
use agnostic_orderbook::error::AoError;
use agnostic_orderbook::state::read_register;
use agnostic_orderbook::state::{OrderSummary, SelfTradeBehavior, Side};
use bonfida_utils::BorshSize;
use bonfida_utils::InstructionsAccount;
use borsh::BorshDeserialize;
use borsh::BorshSerialize;
use bytemuck::{try_from_bytes, Pod, Zeroable};
use num_traits::FromPrimitive;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    program::invoke_signed,
    program_error::{PrintProgramError, ProgramError},
    pubkey::Pubkey,
    rent::Rent,
    system_instruction, system_program,
    sysvar::Sysvar,
};

use super::REFERRAL_MASK;

#[derive(Copy, Clone, Zeroable, Pod, BorshDeserialize, BorshSerialize, BorshSize)]
#[repr(C)]
/**
The required arguments for a new_order instruction.
*/
pub struct Params {
    /// For bids, the min output quantity. For asks, the exact input quantity.
    pub base_qty: u64,
    /// For bids, the exact input quantity. For asks, the min output quantity.
    pub quote_qty: u64,
    /// The maximum number of orders to be matched against.
    ///
    /// Setting this number too high can sometimes lead to excessive resource consumption which can cause a failure.
    pub match_limit: u64,
    /// The order's side (Bid or Ask)
    pub side: u8,
    /// Whether or not the optional discount token account was given
    pub has_discount_token_account: u8,
    /// To eliminate implicit padding
    pub _padding: [u8; 6],
}

#[derive(InstructionsAccount)]
pub struct Accounts<'a, T> {
    /// The SPL token program
    pub spl_token_program: &'a T,

    /// The system program
    pub system_program: &'a T,

    /// The DEX market
    #[cons(writable)]
    pub market: &'a T,

    /// The orderbook
    #[cons(writable)]
    pub orderbook: &'a T,

    /// The AOB event queue
    #[cons(writable)]
    pub event_queue: &'a T,

    /// The AOB bids shared memory
    #[cons(writable)]
    pub bids: &'a T,

    /// The AOB asks shared memory
    #[cons(writable)]
    pub asks: &'a T,

    /// The base token vault
    #[cons(writable)]
    pub base_vault: &'a T,

    /// The quote token vault
    #[cons(writable)]
    pub quote_vault: &'a T,

    /// The DEX market signer
    pub market_signer: &'a T,

    /// The user base token account
    #[cons(writable)]
    pub user_base_account: &'a T,

    /// The user quote token account
    #[cons(writable)]
    pub user_quote_account: &'a T,

    /// The user wallet
    #[cons(writable, signer)]
    pub user_owner: &'a T,

    /// The optional SRM or MSRM discount token account (must be owned by the user wallet)
    pub discount_token_account: Option<&'a T>,

    /// The optional referrer's token account which will receive a 20% cut of the fees
    #[cons(writable)]
    pub fee_referral_account: Option<&'a T>,
}

impl<'a, 'b: 'a> Accounts<'a, AccountInfo<'b>> {
    pub fn parse(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
        has_discount_token_account: bool,
    ) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        let a = Self {
            spl_token_program: next_account_info(accounts_iter)?,
            system_program: next_account_info(accounts_iter)?,
            market: next_account_info(accounts_iter)?,
            orderbook: next_account_info(accounts_iter)?,
            event_queue: next_account_info(accounts_iter)?,
            bids: next_account_info(accounts_iter)?,
            asks: next_account_info(accounts_iter)?,
            base_vault: next_account_info(accounts_iter)?,
            quote_vault: next_account_info(accounts_iter)?,
            market_signer: next_account_info(accounts_iter)?,
            user_base_account: next_account_info(accounts_iter)?,
            user_quote_account: next_account_info(accounts_iter)?,
            user_owner: next_account_info(accounts_iter)?,
            discount_token_account: if has_discount_token_account {
                next_account_info(accounts_iter).ok()
            } else {
                None
            },
            fee_referral_account: next_account_info(accounts_iter).ok(),
        };
        check_signer(a.user_owner).map_err(|e| {
            msg!("The user account owner should be a signer for this transaction!");
            e
        })?;
        check_account_key(
            a.spl_token_program,
            &spl_token::ID,
            DexError::InvalidSplTokenProgram,
        )?;
        check_account_key(
            a.system_program,
            &system_program::ID,
            DexError::InvalidSystemProgramAccount,
        )?;
        if let Some(discount_account) = a.discount_token_account {
            check_account_owner(
                discount_account,
                &spl_token::ID,
                DexError::InvalidSplTokenProgram,
            )?
        }
        check_account_owner(a.market, program_id, DexError::InvalidStateAccountOwner)?;

        Ok(a)
    }
}

pub(crate) fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let Params {
        side,
        base_qty,
        mut quote_qty,
        match_limit,
        has_discount_token_account,
        _padding: _,
    } = try_from_bytes(instruction_data).map_err(|_| ProgramError::InvalidInstructionData)?;
    let accounts = Accounts::parse(program_id, accounts, *has_discount_token_account != 0)?;

    let market_state = DexState::get(accounts.market)?;

    // Check the order size
    if base_qty < &market_state.min_base_order_size {
        msg!("The base order size is too small.");
        return Err(ProgramError::InvalidArgument);
    }

    check_accounts(program_id, &market_state, &accounts).unwrap();
    let fee_tier = accounts
        .discount_token_account
        .map(|a| FeeTier::get(&market_state, a, accounts.user_owner.key))
        .unwrap_or(Ok(FeeTier::Base))?;
    let callback_info = CallBackInfo {
        user_account: Pubkey::default(),
        fee_tier: fee_tier as u8
            | ((accounts.fee_referral_account.is_some() as u8) * REFERRAL_MASK),
    };
    if *side == Side::Bid as u8 {
        // We make sure to leave enough quote quantity to pay for taker fees in the worst case
        quote_qty = fee_tier.remove_taker_fee(quote_qty);
    }

    let orderbook = agnostic_orderbook::state::MarketState::get(accounts.orderbook)?;
    let tick_size = orderbook.tick_size;

    // Transfer the cranking fee to the AAOB program
    let rent = Rent::get()?;
    if accounts.user_owner.lamports()
        < rent.minimum_balance(accounts.user_owner.data_len()) + orderbook.cranker_reward
    {
        msg!("The user does not have enough lamports on his account.");
        return Err(DexError::OutofFunds.into());
    }
    let transfer_cranking_fee = system_instruction::transfer(
        accounts.user_owner.key,
        accounts.orderbook.key,
        orderbook.cranker_reward,
    );
    drop(orderbook);
    invoke(
        &transfer_cranking_fee,
        &[
            accounts.system_program.clone(),
            accounts.user_owner.clone(),
            accounts.orderbook.clone(),
        ],
    )?;

    let (max_base_qty, max_quote_qty, limit_price) = match FromPrimitive::from_u8(*side).unwrap() {
        Side::Bid => (u64::MAX, quote_qty, u64::MAX - (u64::MAX % tick_size)),
        Side::Ask => (*base_qty, u64::MAX, 0),
    };

    let invoke_params = agnostic_orderbook::instruction::new_order::Params {
        max_base_qty,
        max_quote_qty,
        limit_price,
        side: FromPrimitive::from_u8(*side).unwrap(),
        match_limit: *match_limit,
        callback_info: callback_info.try_to_vec()?,
        post_only: false,
        post_allowed: false,
        // No impact as user is Pubkey::default()
        self_trade_behavior: SelfTradeBehavior::DecrementTake,
    };
    let invoke_accounts = agnostic_orderbook::instruction::new_order::Accounts {
        market: accounts.orderbook,
        event_queue: accounts.event_queue,
        bids: accounts.bids,
        asks: accounts.asks,
    };

    if let Err(error) = agnostic_orderbook::instruction::new_order::process(
        program_id,
        invoke_accounts,
        invoke_params,
    ) {
        error.print::<AoError>();
        return Err(DexError::AOBError.into());
    }

    let mut order_summary: OrderSummary = read_register(accounts.event_queue).unwrap().unwrap();

    let referral_fee = fee_tier.referral_fee(order_summary.total_quote_qty);
    let royalties_fees = order_summary
        .total_quote_qty
        .checked_mul(market_state.royalties_bps)
        .unwrap()
        / 10_000;
    let (is_valid, base_transfer_qty, quote_transfer_qty) =
        match FromPrimitive::from_u8(*side).unwrap() {
            Side::Bid => {
                // We update the order summary to properly handle the FOK order type

                order_summary.total_quote_qty +=
                    fee_tier.taker_fee(order_summary.total_quote_qty) + royalties_fees;

                let is_valid = order_summary.total_base_qty >= *base_qty;

                (
                    is_valid,
                    order_summary.total_base_qty,
                    order_summary.total_quote_qty,
                )
            }
            Side::Ask => {
                let taker_fee = fee_tier.taker_fee(order_summary.total_quote_qty);

                let is_valid = order_summary.total_quote_qty >= quote_qty;

                (
                    is_valid,
                    order_summary.total_base_qty,
                    order_summary
                        .total_quote_qty
                        .checked_sub(taker_fee + royalties_fees)
                        .unwrap(),
                )
            }
        };

    if !is_valid {
        msg!("Insufficient output amount");
        return Err(DexError::TransactionAborted.into());
    };

    let base_transfer_params = (
        base_transfer_qty,
        accounts.user_base_account,
        accounts.base_vault,
    );
    let quote_transfer_params = (
        quote_transfer_qty,
        accounts.user_quote_account,
        accounts.quote_vault,
    );

    let (transfer_in_qty, transfer_in_from, transfer_in_to) =
        match FromPrimitive::from_u8(*side).unwrap() {
            Side::Bid => quote_transfer_params,
            Side::Ask => base_transfer_params,
        };

    let transfer_in_instruction = spl_token::instruction::transfer(
        accounts.spl_token_program.key,
        transfer_in_from.key,
        transfer_in_to.key,
        accounts.user_owner.key,
        &[],
        transfer_in_qty,
    )?;

    invoke(
        &transfer_in_instruction,
        &[
            accounts.spl_token_program.clone(),
            transfer_in_from.clone(),
            transfer_in_to.clone(),
            accounts.user_owner.clone(),
        ],
    )?;

    let (transfer_out_qty, transfer_out_to, transfer_out_from) =
        match FromPrimitive::from_u8(*side).unwrap() {
            Side::Bid => base_transfer_params,
            Side::Ask => quote_transfer_params,
        };

    let transfer_out_instruction = spl_token::instruction::transfer(
        accounts.spl_token_program.key,
        transfer_out_from.key,
        transfer_out_to.key,
        accounts.market_signer.key,
        &[],
        transfer_out_qty,
    )?;

    invoke_signed(
        &transfer_out_instruction,
        &[
            accounts.spl_token_program.clone(),
            transfer_out_from.clone(),
            transfer_out_to.clone(),
            accounts.market_signer.clone(),
        ],
        &[&[
            &accounts.market.key.to_bytes(),
            &[market_state.signer_nonce as u8],
        ]],
    )?;

    if let Some(fee_token_account) = accounts.fee_referral_account {
        let referral_fee_transfer_instruction = spl_token::instruction::transfer(
            accounts.spl_token_program.key,
            accounts.quote_vault.key,
            fee_token_account.key,
            accounts.user_owner.key,
            &[],
            referral_fee,
        )?;

        invoke_signed(
            &referral_fee_transfer_instruction,
            &[
                accounts.spl_token_program.clone(),
                accounts.quote_vault.clone(),
                fee_token_account.clone(),
                accounts.user_owner.clone(),
            ],
            &[&[
                &accounts.market.key.to_bytes(),
                &[market_state.signer_nonce as u8],
            ]],
        )?;
    }

    Ok(())
}

fn check_accounts(
    program_id: &Pubkey,
    market_state: &DexState,
    accounts: &Accounts<AccountInfo>,
) -> ProgramResult {
    let market_signer = Pubkey::create_program_address(
        &[
            &accounts.market.key.to_bytes(),
            &[market_state.signer_nonce as u8],
        ],
        program_id,
    )?;
    check_account_key(
        accounts.market_signer,
        &market_signer,
        DexError::InvalidMarketSignerAccount,
    )?;
    check_account_key(
        accounts.orderbook,
        &market_state.orderbook,
        DexError::InvalidOrderbookAccount,
    )?;
    check_account_key(
        accounts.base_vault,
        &market_state.base_vault,
        DexError::InvalidBaseVaultAccount,
    )?;
    check_account_key(
        accounts.quote_vault,
        &market_state.quote_vault,
        DexError::InvalidQuoteVaultAccount,
    )?;

    Ok(())
}
