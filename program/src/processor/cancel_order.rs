//! Cancel an existing order and remove it from the orderbook.
use crate::{
    error::DexError,
    state::{CallBackInfo, DexState, UserAccount},
    utils::{check_account_key, check_account_owner, check_signer},
};
use agnostic_orderbook::{
    error::AoError,
    state::{get_side_from_order_id, Side},
};
use bonfida_utils::BorshSize;
use bonfida_utils::InstructionsAccount;
use borsh::BorshDeserialize;
use borsh::BorshSerialize;
use bytemuck::{CheckedBitPattern, NoUninit};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::{PrintProgramError, ProgramError},
    pubkey::Pubkey,
};

#[derive(Clone, Copy, CheckedBitPattern, NoUninit, BorshDeserialize, BorshSerialize, BorshSize)]
#[repr(C)]
/**
The required arguments for a cancel_order instruction.
*/
pub struct Params {
    /// The order_id of the order to cancel. Redundancy is used here to avoid having to iterate over all
    /// open orders on chain.
    pub order_id: u128,
    /// The index in the user account of the order to cancel
    pub order_index: u64,
    /// Decide wether the `order_id` param is the order id from the user account or a client_order_id which was
    /// given by the user on creation.
    /// The latter means the order_index param will be ignored.
    pub is_client_id: bool,
    pub _padding: [u8; 7],
}

#[derive(InstructionsAccount)]
pub struct Accounts<'a, T> {
    /// The DEX market
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

    /// The DEX user account
    #[cons(writable)]
    pub user: &'a T,

    /// The user wallet
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
        Ok(user_account)
    }
}

pub(crate) fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let params = bytemuck::checked::try_from_bytes(instruction_data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;
    let accounts = Accounts::parse(program_id, accounts)?;

    let Params {
        mut order_id,
        order_index,
        is_client_id,
        _padding,
    } = params;

    let market_state = DexState::get(accounts.market)?;
    let mut user_account_data = accounts.user.data.borrow_mut();
    let mut user_account = accounts.load_user_account(&mut user_account_data)?;

    check_accounts(&market_state, &accounts).unwrap();

    if *is_client_id {
        order_id = user_account.find_order_id_by_client_id(order_id).unwrap();
    } else {
        let order_id_from_index = user_account.read_order(*order_index as usize)?.id;
        if order_id != order_id_from_index {
            msg!("Order id does not match with the order at the given index!");
            return Err(ProgramError::InvalidArgument);
        }
    }

    let invoke_params = agnostic_orderbook::instruction::cancel_order::Params { order_id };
    let invoke_accounts = agnostic_orderbook::instruction::cancel_order::Accounts {
        market: accounts.orderbook,
        event_queue: accounts.event_queue,
        bids: accounts.bids,
        asks: accounts.asks,
    };

    let mut order_summary = match agnostic_orderbook::instruction::cancel_order::process::<
        CallBackInfo,
    >(program_id, invoke_accounts, invoke_params)
    {
        Err(error) => {
            error.print::<AoError>();
            return Err(DexError::AOBError.into());
        }
        Ok(s) => s,
    };
    let side = get_side_from_order_id(order_id);

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
