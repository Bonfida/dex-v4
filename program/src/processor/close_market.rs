use crate::{
    error::DexError,
    state::{AccountTag, DexState},
    utils::{check_account_key, check_account_owner, check_signer},
};
use agnostic_orderbook::error::AoError;
use bonfida_utils::BorshSize;
use bonfida_utils::InstructionsAccount;
use borsh::BorshDeserialize;
use borsh::BorshSerialize;
use bytemuck::{Pod, Zeroable};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::{PrintProgramError, ProgramError},
    program_pack::Pack,
    pubkey::Pubkey,
};
use spl_token::state::Account;

#[derive(Clone, Copy, BorshDeserialize, BorshSerialize, BorshSize, Pod, Zeroable)]
#[repr(C)]
pub struct Params {}

#[derive(InstructionsAccount)]
pub struct Accounts<'a, T> {
    #[cons(writable)]
    market: &'a T,
    #[cons(writable)]
    base_vault: &'a T,
    #[cons(writable)]
    quote_vault: &'a T,
    #[cons(writable)]
    orderbook: &'a T,
    #[cons(writable)]
    event_queue: &'a T,
    #[cons(writable)]
    bids: &'a T,
    #[cons(writable)]
    asks: &'a T,
    #[cons(signer)]
    market_admin: &'a T,
    #[cons(writable)]
    target_lamports_account: &'a T,
}

impl<'a, 'b: 'a> Accounts<'a, AccountInfo<'b>> {
    pub fn parse(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();

        let a = Self {
            market: next_account_info(accounts_iter)?,
            base_vault: next_account_info(accounts_iter)?,
            quote_vault: next_account_info(accounts_iter)?,
            orderbook: next_account_info(accounts_iter)?,
            event_queue: next_account_info(accounts_iter)?,
            bids: next_account_info(accounts_iter)?,
            asks: next_account_info(accounts_iter)?,
            market_admin: next_account_info(accounts_iter)?,
            target_lamports_account: next_account_info(accounts_iter)?,
        };

        check_signer(a.market_admin).map_err(|e| {
            msg!("The market admin should be a signer for this transaction!");
            e
        })?;
        check_account_owner(a.market, program_id, DexError::InvalidStateAccountOwner)?;

        Ok(a)
    }
}

pub(crate) fn process(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let accounts = Accounts::parse(program_id, accounts)?;

    let mut market_state = DexState::get(accounts.market)?;

    check_accounts(&market_state, &accounts).unwrap();

    let base_vault_data = Account::unpack_from_slice(&accounts.base_vault.data.borrow_mut())?;
    let quote_vault_data = Account::unpack_from_slice(&accounts.quote_vault.data.borrow_mut())?;

    if base_vault_data.amount != 0 && quote_vault_data.amount != 0 {
        msg!("Market vaults need to be empty");
        return Err(ProgramError::from(DexError::MarketStillActive));
    }

    if market_state.accumulated_fees != 0 {
        msg!(
            "There are {:?} uncollected fees",
            market_state.accumulated_fees
        );
        return Err(ProgramError::from(DexError::MarketStillActive));
    }

    let invoke_accounts = agnostic_orderbook::instruction::close_market::Accounts {
        market: accounts.orderbook,
        event_queue: accounts.event_queue,
        bids: accounts.bids,
        asks: accounts.asks,
        authority: accounts.market, // No impact with AOB as a lib
        lamports_target_account: accounts.target_lamports_account,
    };
    let invoke_params = agnostic_orderbook::instruction::close_market::Params {};

    if let Err(error) = agnostic_orderbook::instruction::close_market::process(
        program_id,
        invoke_accounts,
        invoke_params,
    ) {
        error.print::<AoError>();
        return Err(DexError::AOBError.into());
    }

    market_state.tag = AccountTag::Closed as u64;

    let mut market_lamports = accounts.market.lamports.borrow_mut();
    let mut base_vault_lamports = accounts.base_vault.lamports.borrow_mut();
    let mut quote_vault_lamports = accounts.quote_vault.lamports.borrow_mut();

    let mut target_lamports = accounts.target_lamports_account.lamports.borrow_mut();

    **target_lamports += **market_lamports + **base_vault_lamports + **quote_vault_lamports;

    **market_lamports = 0;
    **base_vault_lamports = 0;
    **quote_vault_lamports = 0;

    Ok(())
}

fn check_accounts(market_state: &DexState, accounts: &Accounts<AccountInfo>) -> ProgramResult {
    check_account_key(
        accounts.orderbook,
        &market_state.orderbook,
        DexError::InvalidOrderbookAccount,
    )?;
    check_account_key(
        accounts.market_admin,
        &market_state.admin,
        DexError::InvalidMarketAdminAccount,
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
