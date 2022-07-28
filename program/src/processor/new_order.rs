//! Execute a new order instruction. Supported types include Limit, IOC, FOK, or Post only.
use crate::{
    error::DexError,
    state::{CallBackInfo, DexState, FeeTier, Order, UserAccount},
    utils::{check_account_key, check_signer},
    utils::{check_account_owner, fp32_mul},
};
use agnostic_orderbook::error::AoError;
use agnostic_orderbook::state::Side;
use bonfida_utils::BorshSize;
use bonfida_utils::InstructionsAccount;
use borsh::BorshDeserialize;
use borsh::BorshSerialize;
use bytemuck::{try_from_bytes, Pod, Zeroable};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::{PrintProgramError, ProgramError},
    pubkey::Pubkey,
    system_program,
};

use super::REFERRAL_MASK;

#[derive(Copy, Clone, Zeroable, Pod, BorshDeserialize, BorshSerialize, BorshSize)]
#[repr(C)]
/**
The required arguments for a new_order instruction.
*/
pub struct Params {
    #[cfg(all(not(target_arch = "aarch64"), not(feature = "aarch64-test")))]
    /// The client order id number that will be stored in the user account
    pub client_order_id: u128,
    #[cfg(any(target_arch = "aarch64", feature = "aarch64-test"))]
    pub client_order_id: [u64; 2],
    /// The order's limit price (as a FP32)
    pub limit_price: u64,
    /// The max quantity of base token to match and post
    pub max_base_qty: u64,
    /// The max quantity of quote token to match and post
    pub max_quote_qty: u64,
    /// The maximum number of orders to be matched against.
    ///
    /// Setting this number too high can sometimes lead to excessive resource consumption which can cause a failure.
    pub match_limit: u64,
    /// The order's side (Bid or Ask)
    pub side: u8,
    /// The order type (supported types include Limit, FOK, IOC and PostOnly)
    pub order_type: u8,
    /// Configures what happens when this order is at least partially matched against an order belonging to the same user account
    pub self_trade_behavior: u8,
    /// Whether or not the optional discount token account was given
    pub has_discount_token_account: u8,
    /// To eliminate implicit padding
    pub _padding: u32,
}

/// This enum describes all supported order types
#[derive(BorshDeserialize, BorshSerialize, Debug, PartialEq, FromPrimitive)]
pub enum OrderType {
    #[allow(missing_docs)]
    Limit,
    #[allow(missing_docs)]
    ImmediateOrCancel,
    #[allow(missing_docs)]
    FillOrKill,
    #[allow(missing_docs)]
    PostOnly,
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

    /// The DEX user account
    #[cons(writable)]
    pub user: &'a T,

    /// The user source token account
    #[cons(writable)]
    pub user_token_account: &'a T,

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
            user: next_account_info(accounts_iter)?,
            user_token_account: next_account_info(accounts_iter)?,
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
        check_account_owner(a.user, program_id, DexError::InvalidStateAccountOwner)?;
        check_account_owner(a.market, program_id, DexError::InvalidStateAccountOwner)?;

        Ok(a)
    }

    pub fn load_user_account(
        &self,
        user_account_data: &'a mut [u8],
    ) -> Result<UserAccount<'a>, ProgramError> {
        let user_account = UserAccount::from_buffer(user_account_data)?;
        if &user_account.header.owner != self.user_owner.key {
            msg!("Invalid user account owner provided!");
            return Err(ProgramError::InvalidArgument);
        }
        if &user_account.header.market != self.market.key {
            msg!("The provided user account doesn't match the current market");
            return Err(ProgramError::InvalidArgument);
        };
        if &user_account.header.owner != self.user_owner.key {
            msg!("Invalid user account owner provided!");
            return Err(ProgramError::InvalidArgument);
        }
        if &user_account.header.market != self.market.key {
            msg!("The provided user account doesn't match the current market");
            return Err(ProgramError::InvalidArgument);
        };
        Ok(user_account)
    }
}

pub(crate) fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let Params {
        side,
        limit_price,
        max_base_qty,
        mut max_quote_qty,
        order_type,
        self_trade_behavior,
        match_limit,
        has_discount_token_account,
        client_order_id,
        ..
    } = try_from_bytes(instruction_data).map_err(|_| ProgramError::InvalidInstructionData)?;
    #[cfg(any(target_arch = "aarch64", feature = "aarch64-test"))]
    let client_order_id: &u128 = bytemuck::cast_ref(client_order_id);
    let accounts = Accounts::parse(program_id, accounts, *has_discount_token_account != 0)?;

    let market_state = DexState::get(accounts.market)?;
    let mut user_account_data = accounts.user.data.borrow_mut();
    let mut user_account = accounts.load_user_account(&mut user_account_data)?;

    let max_base_qty_scaled = max_base_qty / market_state.base_currency_multiplier;

    // Check the order size
    if max_base_qty < &market_state.min_base_order_size {
        msg!("The base order size is too small.");
        return Err(ProgramError::InvalidArgument);
    }

    check_accounts(&market_state, &accounts).unwrap();
    let (post_only, post_allowed) = match FromPrimitive::from_u8(*order_type).unwrap() {
        OrderType::Limit => (false, true),
        OrderType::ImmediateOrCancel | OrderType::FillOrKill => (false, false),
        OrderType::PostOnly => (true, true),
    };
    let fee_tier = accounts
        .discount_token_account
        .map(|a| FeeTier::get(&market_state, a, accounts.user_owner.key))
        .unwrap_or(Ok(FeeTier::Base))?;
    let callback_info = CallBackInfo {
        user_account: *accounts.user.key,
        fee_tier: fee_tier as u8
            | ((accounts.fee_referral_account.is_some() as u8) * REFERRAL_MASK),
    };
    if *side == Side::Bid as u8 && *order_type != OrderType::PostOnly as u8 {
        // We make sure to leave enough quote quantity to pay for taker fees in the worst case
        max_quote_qty = fee_tier.remove_taker_fee(max_quote_qty);
    }
    let max_quote_qty_scaled = max_quote_qty / market_state.quote_currency_multiplier;

    let invoke_params = agnostic_orderbook::instruction::new_order::Params {
        max_base_qty: max_base_qty_scaled,
        max_quote_qty: max_quote_qty_scaled,
        limit_price: *limit_price,
        side: FromPrimitive::from_u8(*side).unwrap(),
        match_limit: *match_limit,
        callback_info,
        post_only,
        post_allowed,
        self_trade_behavior: FromPrimitive::from_u8(*self_trade_behavior).unwrap(),
    };
    let invoke_accounts = agnostic_orderbook::instruction::new_order::Accounts {
        market: accounts.orderbook,
        event_queue: accounts.event_queue,
        bids: accounts.bids,
        asks: accounts.asks,
    };

    let mut order_summary = match agnostic_orderbook::instruction::new_order::process(
        program_id,
        invoke_accounts,
        invoke_params,
    ) {
        Err(error) => {
            error.print::<AoError>();
            return Err(DexError::AOBError.into());
        }
        Ok(s) => s,
    };

    order_summary.total_base_qty = order_summary
        .total_base_qty
        .checked_mul(market_state.base_currency_multiplier)
        .unwrap();
    order_summary.total_base_qty_posted = order_summary
        .total_base_qty_posted
        .checked_mul(market_state.base_currency_multiplier)
        .unwrap();
    order_summary.total_quote_qty = order_summary
        .total_quote_qty
        .checked_mul(market_state.quote_currency_multiplier)
        .unwrap();

    let (qty_to_transfer, transfer_destination, referral_fee) =
        match FromPrimitive::from_u8(*side).unwrap() {
            Side::Bid => {
                // We update the order summary to properly handle the FOK order type
                let posted_quote_qty =
                    fp32_mul(order_summary.total_base_qty_posted, *limit_price).unwrap();
                let matched_quote_qty = order_summary.total_quote_qty - posted_quote_qty;
                let taker_fee = fee_tier.taker_fee(matched_quote_qty);
                let royalties_fees = matched_quote_qty
                    .checked_mul(market_state.royalties_bps)
                    .unwrap()
                    / 10_000;
                order_summary.total_quote_qty += taker_fee + royalties_fees;
                let referral_fee = fee_tier.referral_fee(matched_quote_qty);
                let q = order_summary
                    .total_quote_qty
                    .saturating_sub(user_account.header.quote_token_free);
                user_account.header.quote_token_free = user_account
                    .header
                    .quote_token_free
                    .saturating_sub(order_summary.total_quote_qty);
                user_account.header.quote_token_locked += posted_quote_qty;
                user_account.header.base_token_free +=
                    order_summary.total_base_qty - order_summary.total_base_qty_posted;

                (q, accounts.quote_vault, referral_fee)
            }
            Side::Ask => {
                let q = order_summary
                    .total_base_qty
                    .saturating_sub(user_account.header.base_token_free);
                user_account.header.base_token_free = user_account
                    .header
                    .base_token_free
                    .saturating_sub(order_summary.total_base_qty);
                user_account.header.base_token_locked += order_summary.total_base_qty_posted;
                let posted_quote_qty =
                    fp32_mul(order_summary.total_base_qty_posted, *limit_price).unwrap();
                let taken_quote_qty = order_summary.total_quote_qty - posted_quote_qty;
                let taker_fee = fee_tier.taker_fee(taken_quote_qty);
                let royalties_fees = taken_quote_qty
                    .checked_mul(market_state.royalties_bps)
                    .unwrap()
                    / 10_000;
                let referral_fee = fee_tier.referral_fee(taken_quote_qty);
                user_account.header.quote_token_free += taken_quote_qty
                    .checked_sub(taker_fee + royalties_fees)
                    .unwrap();
                (q, accounts.base_vault, referral_fee)
            }
        };

    let abort = match FromPrimitive::from_u8(*order_type).unwrap() {
        OrderType::ImmediateOrCancel => order_summary.total_base_qty == 0,
        OrderType::FillOrKill => {
            if *side == Side::Bid as u8 {
                order_summary.total_quote_qty < max_quote_qty
            } else {
                &order_summary.total_base_qty < max_base_qty
            }
        }
        OrderType::PostOnly => order_summary.posted_order_id.is_none(),
        _ => false,
    };

    if abort {
        msg!(
            "The specified order type {:?} has caused an abort",
            order_type
        );
        return Err(DexError::TransactionAborted.into());
    }

    let token_transfer_instruction = spl_token::instruction::transfer(
        accounts.spl_token_program.key,
        accounts.user_token_account.key,
        transfer_destination.key,
        accounts.user_owner.key,
        &[],
        qty_to_transfer,
    )?;

    invoke(
        &token_transfer_instruction,
        &[
            accounts.spl_token_program.clone(),
            accounts.user_token_account.clone(),
            transfer_destination.clone(),
            accounts.user_owner.clone(),
        ],
    )?;

    if let Some(a) = accounts.fee_referral_account {
        let referral_fee_transfer_instruction = spl_token::instruction::transfer(
            accounts.spl_token_program.key,
            accounts.quote_vault.key,
            a.key,
            accounts.user_owner.key,
            &[],
            referral_fee
                .checked_mul(market_state.quote_currency_multiplier)
                .unwrap(),
        )?;

        invoke_signed(
            &referral_fee_transfer_instruction,
            &[
                accounts.spl_token_program.clone(),
                accounts.quote_vault.clone(),
                a.clone(),
                accounts.user_owner.clone(),
            ],
            &[&[
                &accounts.market.key.to_bytes(),
                &[market_state.signer_nonce as u8],
            ]],
        )?;
    }

    if let Some(order_id) = order_summary.posted_order_id {
        user_account.add_order(Order {
            id: order_id,
            client_id: *client_order_id,
        })?;
        msg!("Added new order with order_id {:?}", order_id);
    }

    user_account.header.accumulated_taker_base_volume += order_summary
        .total_base_qty
        .saturating_sub(order_summary.total_base_qty_posted);
    user_account.header.accumulated_taker_quote_volume += order_summary
        .total_quote_qty
        .saturating_sub(fp32_mul(order_summary.total_base_qty_posted, *limit_price).unwrap());

    Ok(())
}

fn check_accounts(market_state: &DexState, accounts: &Accounts<AccountInfo>) -> ProgramResult {
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
