use std::rc::Rc;

use agnostic_orderbook::state::{
    get_side_from_order_id, EventQueue, EventQueueHeader, OrderSummary, Side,
};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{
    error::DexError,
    state::{DexState, UserAccount},
    utils::{check_account_key, check_account_owner, check_signer},
};

use super::CALLBACK_INFO_LEN;

#[derive(BorshDeserialize, BorshSerialize)]
/**
The required arguments for a cancel_order instruction.
*/
pub struct Params {
    /// The index in the user account of the order to cancel
    pub order_index: u64,
    /// The order_id of the order to cancel. Redundancy is used here to avoid having to iterate over all
    /// open orders on chain.
    pub order_id: u128,
}

struct Accounts<'a, 'b: 'a> {
    aaob_program: &'a AccountInfo<'b>,
    market: &'a AccountInfo<'b>,
    market_signer: &'a AccountInfo<'b>,
    orderbook: &'a AccountInfo<'b>,
    event_queue: &'a AccountInfo<'b>,
    bids: &'a AccountInfo<'b>,
    asks: &'a AccountInfo<'b>,
    user: &'a AccountInfo<'b>,
    user_owner: &'a AccountInfo<'b>,
}

impl<'a, 'b: 'a> Accounts<'a, 'b> {
    pub fn parse(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        let a = Self {
            aaob_program: next_account_info(accounts_iter)?,
            market: next_account_info(accounts_iter)?,
            market_signer: next_account_info(accounts_iter)?,
            orderbook: next_account_info(accounts_iter)?,
            event_queue: next_account_info(accounts_iter)?,
            bids: next_account_info(accounts_iter)?,
            asks: next_account_info(accounts_iter)?,
            user: next_account_info(accounts_iter)?,
            user_owner: next_account_info(accounts_iter)?,
        };
        check_signer(&a.user_owner).map_err(|e| {
            msg!("The user account owner should be a signer for this transaction!");
            e
        })?;
        check_account_key(
            &a.aaob_program,
            &agnostic_orderbook::ID,
            DexError::InvalidAobProgramAccount,
        )?;
        check_account_owner(a.market, program_id, DexError::InvalidStateAccountOwner)?;
        check_account_owner(a.user, program_id, DexError::InvalidStateAccountOwner)?;

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
        order_index,
        order_id,
    } = params;

    let market_state =
        DexState::deserialize(&mut (&accounts.market.data.borrow() as &[u8]))?.check()?;
    let mut user_account = accounts.load_user_account()?;

    check_accounts(program_id, &market_state, &accounts).unwrap();

    let order_id_from_index = user_account.read_order(order_index as usize)?;

    if order_id != order_id_from_index {
        msg!("Order id does not match with the order at the given index!");
        return Err(ProgramError::InvalidArgument);
    }

    let cancel_order_instruction = agnostic_orderbook::instruction::cancel_order(
        *accounts.aaob_program.key,
        *accounts.orderbook.key,
        *accounts.market_signer.key,
        *accounts.event_queue.key,
        *accounts.bids.key,
        *accounts.asks.key,
        agnostic_orderbook::instruction::cancel_order::Params { order_id },
    );

    invoke_signed(
        &cancel_order_instruction,
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

    let order_summary: OrderSummary = event_queue.read_register().unwrap().unwrap();

    let side = get_side_from_order_id(order_id);

    match side {
        Side::Bid => {
            user_account.header.quote_token_free = user_account
                .header
                .quote_token_free
                .checked_add(order_summary.total_quote_qty)
                .unwrap();
            user_account.header.quote_token_locked = user_account
                .header
                .quote_token_locked
                .checked_sub(order_summary.total_quote_qty)
                .unwrap();
        }
        Side::Ask => {
            user_account.header.base_token_free = user_account
                .header
                .base_token_free
                .checked_add(order_summary.total_base_qty)
                .unwrap();
            user_account.header.base_token_locked = user_account
                .header
                .base_token_locked
                .checked_sub(order_summary.total_base_qty)
                .unwrap();
        }
    };

    user_account.remove_order(order_index as usize)?;

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

    Ok(())
}
