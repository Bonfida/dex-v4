use std::rc::Rc;

use agnostic_orderbook::{
    state::{EventQueue, EventQueueHeader, OrderSummary, SelfTradeBehavior, Side},
    CRANKER_REWARD,
};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction, system_program,
    sysvar::Sysvar,
};

use crate::{
    error::DexError,
    state::{AccountTag, CallBackInfo, DexState, FeeTier, UserAccount},
    utils::{check_account_key, check_signer},
    utils::{check_account_owner, fp32_mul},
};

use super::CALLBACK_INFO_LEN;

#[derive(BorshDeserialize, BorshSerialize)]
/**
The required arguments for a new_order instruction.
*/
pub struct Params {
    /// The order's side (Bid or Ask)
    pub side: Side,
    /// The order's limit price (as a FP32)
    pub limit_price: u64,
    /// The max quantity of base token to match and post
    pub max_base_qty: u64,
    /// The max quantity of quote token to match and post
    pub max_quote_qty: u64,
    /// The order type (supported types include Limit, FOK, IOC and PostOnly)
    pub order_type: OrderType,
    /// Configures what happens when this order is at least partially matched against an order belonging to the same user account
    pub self_trade_behavior: SelfTradeBehavior,
    /// The maximum number of orders to be matched against.
    ///
    /// Setting this number too high can sometimes lead to excessive resource consumption which can cause a failure.
    pub match_limit: u64,
}

/// This enum describes all supported order types
#[derive(BorshDeserialize, BorshSerialize, Debug, PartialEq)]
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

struct Accounts<'a, 'b: 'a> {
    aaob_program: &'a AccountInfo<'b>,
    spl_token_program: &'a AccountInfo<'b>,
    system_program: &'a AccountInfo<'b>,
    rent_sysvar: &'a AccountInfo<'b>,
    market: &'a AccountInfo<'b>,
    market_signer: &'a AccountInfo<'b>,
    orderbook: &'a AccountInfo<'b>,
    event_queue: &'a AccountInfo<'b>,
    bids: &'a AccountInfo<'b>,
    asks: &'a AccountInfo<'b>,
    base_vault: &'a AccountInfo<'b>,
    quote_vault: &'a AccountInfo<'b>,
    user: &'a AccountInfo<'b>,
    user_token_account: &'a AccountInfo<'b>,
    user_owner: &'a AccountInfo<'b>,
    discount_token_account: Option<&'a AccountInfo<'b>>,
}

impl<'a, 'b: 'a> Accounts<'a, 'b> {
    pub fn parse(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        let a = Self {
            aaob_program: next_account_info(accounts_iter)?,
            spl_token_program: next_account_info(accounts_iter)?,
            system_program: next_account_info(accounts_iter)?,
            rent_sysvar: next_account_info(accounts_iter)?,
            market: next_account_info(accounts_iter)?,
            market_signer: next_account_info(accounts_iter)?,
            orderbook: next_account_info(accounts_iter)?,
            event_queue: next_account_info(accounts_iter)?,
            bids: next_account_info(accounts_iter)?,
            asks: next_account_info(accounts_iter)?,
            base_vault: next_account_info(accounts_iter)?,
            quote_vault: next_account_info(accounts_iter)?,
            user: next_account_info(accounts_iter)?,
            user_token_account: next_account_info(accounts_iter)?,
            user_owner: next_account_info(accounts_iter)?,
            discount_token_account: next_account_info(accounts_iter).ok(),
        };
        check_signer(&a.user_owner).unwrap();
        check_account_key(a.spl_token_program, &spl_token::id()).unwrap();
        check_account_key(a.system_program, &system_program::id()).unwrap();
        check_account_owner(a.user, program_id).unwrap();

        Ok(a)
    }

    pub fn load_user_account(&self) -> Result<UserAccount<'b>, ProgramError> {
        let user_account =
            match AccountTag::deserialize(&mut (&self.user.data.borrow() as &[u8])).unwrap() {
                AccountTag::UserAccount => {
                    let u = UserAccount::parse(&self.user)?;
                    if &u.header.owner != self.user_owner.key {
                        msg!("Invalid user account owner provided!");
                        return Err(ProgramError::InvalidArgument);
                    }
                    if &u.header.market != self.market.key {
                        msg!("The provided user account doesn't match the current market");
                        return Err(ProgramError::InvalidArgument);
                    };
                    u
                }
                AccountTag::Uninitialized => {
                    msg!("Invalid user account!");
                    return Err(ProgramError::InvalidArgument);
                }
                _ => return Err(ProgramError::InvalidArgument),
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
    params: Params,
) -> ProgramResult {
    let accounts = Accounts::parse(program_id, accounts)?;

    let Params {
        side,
        limit_price,
        max_base_qty,
        mut max_quote_qty,
        order_type,
        self_trade_behavior,
        match_limit,
    } = params;
    let market_state =
        DexState::deserialize(&mut (&accounts.market.data.borrow() as &[u8]))?.check()?;
    let mut user_account = accounts.load_user_account()?;
    let mut market_data: &mut [u8] = &mut accounts.market.data.borrow_mut();
    market_state.serialize(&mut market_data).unwrap();

    // Check the order size
    if max_base_qty < market_state.min_base_order_size {
        msg!("The base order size is too small.");
        return Err(ProgramError::InvalidArgument);
    }

    check_accounts(program_id, &market_state, &accounts).unwrap();
    let (post_only, post_allowed) = match order_type {
        OrderType::Limit => (false, true),
        OrderType::ImmediateOrCancel | OrderType::FillOrKill => (false, false),
        OrderType::PostOnly => (true, true),
    };
    let callback_info = CallBackInfo {
        user_account: *accounts.user.key,
        fee_tier: accounts
            .discount_token_account
            .map(|a| FeeTier::get(a, accounts.user_owner.key))
            .unwrap_or(Ok(FeeTier::Base))?,
    };
    if side == Side::Bid && order_type != OrderType::PostOnly {
        // We make sure to leave enough quote quantity to pay for taker fees in the worst case
        max_quote_qty = callback_info.fee_tier.remove_taker_fee(max_quote_qty);
    }

    //Transfer the cranking fee to the AAOB program
    let rent = Rent::from_account_info(accounts.rent_sysvar)?;
    if accounts.user_owner.lamports()
        < rent.minimum_balance(accounts.user_owner.data_len()) + CRANKER_REWARD
    {
        msg!("The user does not have enough lamports on his account.");
        return Err(DexError::OutofFunds.into());
    }
    let transfer_cranking_fee = system_instruction::transfer(
        accounts.user_owner.key,
        accounts.orderbook.key,
        agnostic_orderbook::CRANKER_REWARD,
    );
    invoke(
        &transfer_cranking_fee,
        &[
            accounts.system_program.clone(),
            accounts.user_owner.clone(),
            accounts.orderbook.clone(),
        ],
    )?;

    let new_order_instruction = agnostic_orderbook::instruction::new_order(
        *accounts.aaob_program.key,
        *accounts.orderbook.key,
        *accounts.market_signer.key,
        *accounts.event_queue.key,
        *accounts.bids.key,
        *accounts.asks.key,
        agnostic_orderbook::instruction::new_order::Params {
            max_base_qty,
            max_quote_qty,
            limit_price,
            side,
            match_limit,
            callback_info: callback_info.try_to_vec()?,
            post_only,
            post_allowed,
            self_trade_behavior,
        },
    );
    invoke_signed(
        &new_order_instruction,
        &[
            accounts.aaob_program.clone(),
            accounts.orderbook.clone(),
            accounts.event_queue.clone(),
            accounts.bids.clone(),
            accounts.asks.clone(),
            accounts.market_signer.clone(),
        ],
        &[&[
            &accounts.market.key.to_bytes(),
            &[market_state.signer_nonce],
        ]],
    )?;
    let event_queue_header =
        EventQueueHeader::deserialize(&mut (&accounts.event_queue.data.borrow() as &[u8]))?;
    let event_queue = EventQueue::new(
        event_queue_header,
        Rc::clone(&accounts.event_queue.data),
        CALLBACK_INFO_LEN as usize,
    );

    let mut order_summary: OrderSummary = event_queue.read_register().unwrap().unwrap();

    if side == Side::Bid {
        order_summary.total_quote_qty += callback_info.fee_tier.taker_fee(
            order_summary.total_quote_qty - fp32_mul(order_summary.total_base_qty, limit_price), //TODO check
        )
    }

    let abort = match order_type {
        OrderType::ImmediateOrCancel => order_summary.total_base_qty == 0,
        OrderType::FillOrKill => {
            (order_summary.total_base_qty == max_base_qty)
                || (order_summary.total_quote_qty == max_quote_qty)
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

    let (qty_to_transfer, transfer_destination) = match side {
        Side::Bid => {
            let q = order_summary
                .total_quote_qty
                .saturating_sub(user_account.header.quote_token_free);
            user_account.header.quote_token_free -= std::cmp::min(
                order_summary.total_quote_qty,
                user_account.header.quote_token_free,
            );
            let posted_quote_qty = (order_summary.total_base_qty_posted * limit_price) >> 32;
            user_account.header.quote_token_locked += posted_quote_qty;
            user_account.header.base_token_free +=
                order_summary.total_base_qty - order_summary.total_base_qty_posted;
            (q, accounts.quote_vault)
        }
        Side::Ask => {
            let q = order_summary
                .total_base_qty
                .saturating_sub(user_account.header.base_token_free);
            user_account.header.base_token_free -= std::cmp::min(
                order_summary.total_base_qty,
                user_account.header.base_token_free,
            );
            user_account.header.base_token_locked += order_summary.total_base_qty_posted;
            let posted_quote_qty = (order_summary.total_base_qty_posted * limit_price) >> 32;
            let taken_quote_qty = order_summary.total_quote_qty - posted_quote_qty;
            let taker_fee = callback_info.fee_tier.taker_fee(taken_quote_qty);
            user_account.header.quote_token_free += taken_quote_qty - taker_fee;
            (q, accounts.base_vault)
        }
    };

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

    if let Some(order_id) = order_summary.posted_order_id {
        user_account.add_order(order_id)?;
        msg!("Added new order with order_id {:?}", order_id);
    }

    user_account.write();

    Ok(())
}

fn check_accounts(
    program_id: &Pubkey,
    market_state: &DexState,
    accounts: &Accounts,
) -> ProgramResult {
    let market_signer = Pubkey::create_program_address(
        &[
            &accounts.market.key.to_bytes(),
            &[market_state.signer_nonce],
        ],
        program_id,
    )?;
    check_account_key(accounts.market_signer, &market_signer).unwrap();
    check_account_key(accounts.orderbook, &market_state.orderbook).unwrap();
    check_account_key(accounts.base_vault, &market_state.base_vault).unwrap();
    check_account_key(accounts.quote_vault, &market_state.quote_vault).unwrap();
    check_account_key(accounts.aaob_program, &market_state.aaob_program).unwrap();

    Ok(())
}
