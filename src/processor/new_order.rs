use std::rc::Rc;

use agnostic_orderbook::state::{
    EventQueue, EventQueueHeader, OrderSummary, SelfTradeBehavior, Side,
};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{
    error::DexError,
    state::{DexState, UserAccount},
    utils::{check_account_key, check_signer},
};

#[derive(BorshDeserialize, BorshSerialize)]
/**
The required arguments for a create_market instruction.
*/
pub struct Params {
    side: Side,
    limit_price: u64,
    max_base_qty: u64,
    max_quote_qty: u64,
    order_type: OrderType,
    self_trade_behavior: SelfTradeBehavior,
    match_limit: u64,
}

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub enum OrderType {
    Limit,
    ImmediateOrCancel,
    FillOrKill,
    PostOnly,
}

struct Accounts<'a, 'b: 'a> {
    aaob_program: &'a AccountInfo<'b>,
    spl_token_program: &'a AccountInfo<'b>,
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
}

impl<'a, 'b: 'a> Accounts<'a, 'b> {
    pub fn parse(
        _program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        let a = Self {
            aaob_program: next_account_info(accounts_iter)?,
            spl_token_program: next_account_info(accounts_iter)?,
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
        };
        check_signer(&a.user_owner).unwrap();
        check_account_key(a.spl_token_program, &spl_token::id()).unwrap();

        Ok(a)
    }

    pub fn load_user_account(&self) -> Result<UserAccount<'b>, ProgramError> {
        let user_account = UserAccount::parse(&self.user)?;
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
        max_quote_qty,
        order_type,
        self_trade_behavior,
        match_limit,
    } = params;

    let market_state =
        DexState::deserialize(&mut (&accounts.market.data.borrow() as &[u8]))?.check()?;

    let mut user_account = accounts.load_user_account()?;

    let mut market_data: &mut [u8] = &mut accounts.market.data.borrow_mut();
    market_state.serialize(&mut market_data).unwrap();

    check_accounts(program_id, &market_state, &accounts).unwrap();

    let (post_only, post_allowed) = match order_type {
        OrderType::Limit => (false, true),
        OrderType::ImmediateOrCancel | OrderType::FillOrKill => (false, false),
        OrderType::PostOnly => (true, true),
    };

    let new_order_instruction = agnostic_orderbook::instruction::new_order(
        *accounts.aaob_program.key,
        *accounts.orderbook.key,
        *accounts.market_signer.key,
        *accounts.event_queue.key,
        *accounts.bids.key,
        *accounts.asks.key,
        agnostic_orderbook::instruction::new_order::Params {
            max_asset_qty: max_base_qty,
            max_quote_qty,
            limit_price,
            side,
            match_limit,
            callback_info: accounts.user.key.to_bytes().to_vec(),
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
        32,
    );

    let order_summary: OrderSummary = event_queue.read_register().unwrap().unwrap();

    let abort = match order_type {
        OrderType::ImmediateOrCancel => order_summary.total_asset_qty == 0,
        OrderType::FillOrKill => {
            (order_summary.total_asset_qty == max_base_qty)
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
            user_account.header.quote_token_locked += order_summary.total_quote_qty;
            (q, accounts.quote_vault)
        }
        Side::Ask => {
            let q = order_summary
                .total_asset_qty
                .saturating_sub(user_account.header.base_token_free);
            user_account.header.base_token_free -= std::cmp::min(
                order_summary.total_asset_qty,
                user_account.header.base_token_free,
            );
            user_account.header.base_token_locked += order_summary.total_asset_qty;
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
            accounts.user.clone(),
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
    Ok(())
}
