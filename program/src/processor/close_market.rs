//! Close an existing market
use crate::{
    error::DexError,
    state::{AccountTag, CallBackInfo, DexState},
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
    program::invoke_signed,
    program_error::{PrintProgramError, ProgramError},
    program_pack::Pack,
    pubkey::Pubkey,
};
use spl_token::instruction::close_account;
use spl_token::state::Account;

#[derive(Clone, Copy, BorshDeserialize, BorshSerialize, BorshSize, Pod, Zeroable)]
#[repr(C)]
pub struct Params {}

#[derive(InstructionsAccount)]
pub struct Accounts<'a, T> {
    /// The market account
    #[cons(writable)]
    pub market: &'a T,

    /// The market base vault account
    #[cons(writable)]
    pub base_vault: &'a T,

    /// The market quote vault account
    #[cons(writable)]
    pub quote_vault: &'a T,

    /// The AOB orderbook account
    #[cons(writable)]
    pub orderbook: &'a T,

    /// The AOB event queue account
    #[cons(writable)]
    pub event_queue: &'a T,

    /// The AOB bids account
    #[cons(writable)]
    pub bids: &'a T,

    /// The AOB asks account
    #[cons(writable)]
    pub asks: &'a T,

    /// The makret admin account
    #[cons(signer)]
    pub market_admin: &'a T,

    /// The target lamports account
    #[cons(writable)]
    pub target_lamports_account: &'a T,

    /// The market signer
    pub market_signer: &'a T,

    /// The SPL token program ID
    pub spl_token_program: &'a T,
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
            market_signer: next_account_info(accounts_iter)?,
            spl_token_program: next_account_info(accounts_iter)?,
        };

        // Check keys
        check_account_key(
            a.spl_token_program,
            &spl_token::ID,
            DexError::InvalidStateAccountOwner,
        )?;

        // Check owners
        check_account_owner(a.market, program_id, DexError::InvalidStateAccountOwner)?;

        // Check signers
        check_signer(a.market_admin).map_err(|e| {
            msg!("The market admin should be a signer for this transaction!");
            e
        })?;

        Ok(a)
    }
}

pub(crate) fn process(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let accounts = Accounts::parse(program_id, accounts)?;

    let mut market_state = DexState::get(accounts.market)?;

    check_accounts(program_id, &market_state, &accounts).unwrap();

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
        lamports_target_account: accounts.target_lamports_account,
    };
    let invoke_params = agnostic_orderbook::instruction::close_market::Params {};

    if let Err(error) = agnostic_orderbook::instruction::close_market::process::<CallBackInfo>(
        program_id,
        invoke_accounts,
        invoke_params,
    ) {
        error.print::<AoError>();
        return Err(DexError::AOBError.into());
    }

    market_state.tag = AccountTag::Closed as u64;
    let nonce = market_state.signer_nonce;
    drop(market_state);

    // Close token accounts
    let ix = close_account(
        &spl_token::ID,
        accounts.base_vault.key,
        accounts.market.key,
        accounts.market_signer.key,
        &[],
    )?;
    invoke_signed(
        &ix,
        &[
            accounts.spl_token_program.clone(),
            accounts.base_vault.clone(),
            accounts.market.clone(),
            accounts.market_signer.clone(),
        ],
        &[&[&accounts.market.key.to_bytes(), &[nonce]]],
    )?;
    let ix = close_account(
        &spl_token::ID,
        accounts.quote_vault.key,
        accounts.market.key,
        accounts.market_signer.key,
        &[],
    )?;
    invoke_signed(
        &ix,
        &[
            accounts.spl_token_program.clone(),
            accounts.quote_vault.clone(),
            accounts.market.clone(),
            accounts.market_signer.clone(),
        ],
        &[&[&accounts.market.key.to_bytes(), &[nonce]]],
    )?;

    let mut market_lamports = accounts.market.lamports.borrow_mut();
    let mut target_lamports = accounts.target_lamports_account.lamports.borrow_mut();

    **target_lamports += **market_lamports;

    **market_lamports = 0;

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
