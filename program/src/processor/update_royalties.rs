//! Update market royalties.
use {
    agnostic_orderbook::state::{EventQueueHeader, MarketState, EVENT_QUEUE_HEADER_LEN},
    bonfida_utils::{
        checks::{check_account_key, check_account_owner},
        BorshSize, InstructionsAccount,
    },
    borsh::{BorshDeserialize, BorshSerialize},
    bytemuck::{Pod, Zeroable},
    mpl_token_metadata::state::Metadata,
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program_error::ProgramError,
        pubkey::Pubkey,
    },
};

use crate::{
    error::DexError,
    state::DexState,
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
    let aob_state = MarketState::get(accounts.orderbook)?;

    check_metadata_account(accounts.token_metadata, &market_state.base_mint)?;
    check_account_key(accounts.orderbook, &market_state.orderbook)?;

    if aob_state.event_queue != accounts.event_queue.key.to_bytes() {
        return Err(DexError::EventQueueMismatch.into());
    }

    let header = {
        let mut event_queue_data: &[u8] =
            &accounts.event_queue.data.borrow()[0..EVENT_QUEUE_HEADER_LEN];
        EventQueueHeader::deserialize(&mut event_queue_data).unwrap()
    };

    if header.count != 0 {
        msg!("Header {}", header.count);
        return Err(DexError::EventQueueMustBeEmpty.into());
    }

    let metadata = Metadata::from_account_info(accounts.token_metadata)?;
    verify_metadata(&metadata.data.creators.unwrap())?;

    market_state.royalties_bps = metadata.data.seller_fee_basis_points as u64;

    Ok(())
}
