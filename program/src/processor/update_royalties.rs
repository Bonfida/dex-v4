//! Update market royalties.
use {
    bonfida_utils::{
        checks::{check_account_key, check_account_owner},
        BorshSize, InstructionsAccount,
    },
    borsh::{BorshDeserialize, BorshSerialize},
    bytemuck::{Pod, Zeroable},
    mpl_token_metadata::state::{Metadata, TokenMetadataAccount},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program_error::ProgramError,
        pubkey::Pubkey,
    },
};

use asset_agnostic_orderbook::state::{event_queue::EventQueue, AccountTag};

use crate::{
    error::DexError,
    state::{CallBackInfo, DexState},
    utils::{check_metadata_account, verify_metadata},
};

#[derive(Copy, Clone, Zeroable, Pod, BorshDeserialize, BorshSerialize, BorshSize)]
#[repr(C)]
pub struct Params {}

#[derive(InstructionsAccount)]
pub struct Accounts<'a, T> {
    /// The DEX market
    #[cons(writable)]
    pub market: &'a T,

    /// The event queue account
    pub event_queue: &'a T,

    /// The AOB market account
    pub orderbook: &'a T,

    /// The token metadata
    pub token_metadata: &'a T,
}

impl<'a, 'b: 'a> Accounts<'a, AccountInfo<'b>> {
    pub(crate) fn parse(
        accounts: &'a [AccountInfo<'b>],
        program_id: &Pubkey,
    ) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        let a = Self {
            market: next_account_info(accounts_iter)?,
            event_queue: next_account_info(accounts_iter)?,
            orderbook: next_account_info(accounts_iter)?,
            token_metadata: next_account_info(accounts_iter)?,
        };

        // Check keys

        // Check owners
        check_account_owner(a.market, program_id)?;
        check_account_owner(a.orderbook, program_id)?;
        check_account_owner(a.event_queue, program_id)?;

        // Check signers

        Ok(a)
    }
}

pub(crate) fn process(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let accounts = Accounts::parse(accounts, program_id)?;

    let mut market_state = DexState::get(accounts.market)?;
    let mut orderbook_guard = accounts.orderbook.data.borrow_mut();
    let aob_state = asset_agnostic_orderbook::state::market_state::MarketState::from_buffer(
        &mut orderbook_guard,
        AccountTag::Market,
    )?;

    check_metadata_account(accounts.token_metadata, &market_state.base_mint)?;
    check_account_key(accounts.orderbook, &market_state.orderbook)?;

    if &aob_state.event_queue != accounts.event_queue.key {
        return Err(DexError::EventQueueMismatch.into());
    }

    let mut event_queue_guard = accounts.event_queue.data.borrow_mut();

    let event_queue =
        EventQueue::<CallBackInfo>::from_buffer(&mut event_queue_guard, AccountTag::EventQueue)?;

    if !event_queue.is_empty() {
        msg!("Header {}", event_queue.len());
        return Err(DexError::EventQueueMustBeEmpty.into());
    }

    let metadata: Metadata = Metadata::from_account_info(accounts.token_metadata)?;
    verify_metadata(&metadata.data.creators.unwrap())?;

    market_state.royalties_bps = metadata.data.seller_fee_basis_points as u64;

    Ok(())
}
