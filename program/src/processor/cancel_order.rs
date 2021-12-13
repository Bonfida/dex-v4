use std::rc::Rc;

use crate::{
    error::DexError,
    state::{DexState, UserAccount},
    utils::{check_account_key, check_account_owner, check_signer},
};
use agnostic_orderbook::{
    error::AoError,
    state::{get_side_from_order_id, EventQueue, EventQueueHeader, OrderSummary, Side},
};
use bonfida_utils::BorshSize;
use bonfida_utils::InstructionsAccount;
use borsh::BorshDeserialize;
use borsh::BorshSerialize;
use bytemuck::{try_from_bytes, Pod, Zeroable};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::{PrintProgramError, ProgramError},
    pubkey::Pubkey,
};

use super::CALLBACK_INFO_LEN;

#[derive(Clone, Copy, Zeroable, Pod, BorshDeserialize, BorshSerialize, BorshSize)]
#[repr(C)]
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

#[derive(InstructionsAccount)]
pub struct Accounts<'a, T> {
    pub market: &'a T,
    #[cons(writable)]
    pub orderbook: &'a T,
    #[cons(writable)]
    pub event_queue: &'a T,
    #[cons(writable)]
    pub bids: &'a T,
    #[cons(writable)]
    pub asks: &'a T,
    #[cons(writable)]
    pub user: &'a T,
    #[cons(signer)]
    pub user_owner: &'a T,
}

impl<'a, 'b: 'a> Accounts<'a, AccountInfo<'b>> {
    pub fn parse(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        let a = Self {
            market: next_account_info(accounts_iter)?,
            orderbook: next_account_info(accounts_iter)?,
            event_queue: next_account_info(accounts_iter)?,
            bids: next_account_info(accounts_iter)?,
            asks: next_account_info(accounts_iter)?,
            user: next_account_info(accounts_iter)?,
            user_owner: next_account_info(accounts_iter)?,
        };
        check_signer(a.user_owner).map_err(|e| {
            msg!("The user account owner should be a signer for this transaction!");
            e
        })?;
        check_account_owner(a.market, program_id, DexError::InvalidStateAccountOwner)?;
        check_account_owner(a.user, program_id, DexError::InvalidStateAccountOwner)?;

        Ok(a)
    }

    pub fn load_user_account(&self) -> Result<UserAccount<'a>, ProgramError> {
        let user_account = UserAccount::get(self.user)?;
        if user_account.header.owner != self.user_owner.key.to_bytes() {
            msg!("Invalid user account owner provided!");
            return Err(ProgramError::InvalidArgument);
        }
        if user_account.header.market != self.market.key.to_bytes() {
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
    let params =
        try_from_bytes(instruction_data).map_err(|_| ProgramError::InvalidInstructionData)?;
    let accounts = Accounts::parse(program_id, accounts)?;

    let Params {
        order_index,
        order_id,
    } = params;

    let market_state = DexState::get(accounts.market)?;
    let mut user_account = accounts.load_user_account()?;

    check_accounts(&market_state, &accounts).unwrap();

    let order_id_from_index = user_account.read_order(*order_index as usize)?;

    if order_id != &order_id_from_index {
        msg!("Order id does not match with the order at the given index!");
        return Err(ProgramError::InvalidArgument);
    }

    let invoke_params = agnostic_orderbook::instruction::cancel_order::Params {
        order_id: *order_id,
    };
    let invoke_accounts = agnostic_orderbook::instruction::cancel_order::Accounts {
        market: accounts.orderbook,
        event_queue: accounts.event_queue,
        bids: accounts.bids,
        asks: accounts.asks,
        authority: accounts.market, // No impact with AOB as a lib
    };

    if let Err(error) = agnostic_orderbook::instruction::cancel_order::process(
        program_id,
        invoke_accounts,
        invoke_params,
    ) {
        error.print::<AoError>();
        return Err(DexError::AOBError.into());
    }

    let event_queue_header =
        EventQueueHeader::deserialize(&mut (&accounts.event_queue.data.borrow() as &[u8]))?;
    let event_queue = EventQueue::new(
        event_queue_header,
        Rc::clone(&accounts.event_queue.data),
        CALLBACK_INFO_LEN as usize,
    );

    let order_summary: OrderSummary = event_queue.read_register().unwrap().unwrap();

    let side = get_side_from_order_id(*order_id);

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

    user_account.remove_order(*order_index as usize)?;

    Ok(())
}

fn check_accounts(market_state: &DexState, accounts: &Accounts<AccountInfo>) -> ProgramResult {
    check_account_key(
        accounts.orderbook,
        &market_state.orderbook,
        DexError::InvalidOrderbookAccount,
    )?;

    Ok(())
}
