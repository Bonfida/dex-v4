use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
};
use spl_token::state::Account;

use crate::{
    error::DexError,
    state::{AccountTag, DexState},
    utils::{check_account_key, check_signer},
};

#[derive(BorshDeserialize, BorshSerialize)]
/**
The required arguments for a close_market instruction.
*/
pub struct Params {}

struct Accounts<'a, 'b: 'a> {
    market: &'a AccountInfo<'b>,
    base_vault: &'a AccountInfo<'b>,
    quote_vault: &'a AccountInfo<'b>,
    market_signer: &'a AccountInfo<'b>,
    orderbook: &'a AccountInfo<'b>,
    event_queue: &'a AccountInfo<'b>,
    bids: &'a AccountInfo<'b>,
    asks: &'a AccountInfo<'b>,
    aaob_program: &'a AccountInfo<'b>,
    market_admin: &'a AccountInfo<'b>,
    target_lamports_account: &'a AccountInfo<'b>,
}

impl<'a, 'b: 'a> Accounts<'a, 'b> {
    pub fn parse(accounts: &'a [AccountInfo<'b>]) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();

        let a = Self {
            market: next_account_info(accounts_iter)?,
            base_vault: next_account_info(accounts_iter)?,
            quote_vault: next_account_info(accounts_iter)?,
            market_signer: next_account_info(accounts_iter)?,
            orderbook: next_account_info(accounts_iter)?,
            event_queue: next_account_info(accounts_iter)?,
            bids: next_account_info(accounts_iter)?,
            asks: next_account_info(accounts_iter)?,
            aaob_program: next_account_info(accounts_iter)?,
            market_admin: next_account_info(accounts_iter)?,
            target_lamports_account: next_account_info(accounts_iter)?,
        };

        check_signer(&a.market_admin).unwrap();

        Ok(a)
    }
}

pub(crate) fn process(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let accounts = Accounts::parse(accounts)?;

    let mut market_state =
        DexState::deserialize(&mut (&accounts.market.data.borrow_mut() as &[u8]))?.check()?;

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

    let close_aaob_market_instruction = agnostic_orderbook::instruction::close_market(
        *accounts.aaob_program.key,
        *accounts.market.key,
        *accounts.event_queue.key,
        *accounts.bids.key,
        *accounts.asks.key,
        *accounts.market_signer.key,
        *accounts.target_lamports_account.key,
    );

    invoke_signed(
        &close_aaob_market_instruction,
        &[
            accounts.market.clone(),
            accounts.event_queue.clone(),
            accounts.bids.clone(),
            accounts.asks.clone(),
            accounts.market_signer.clone(),
            accounts.target_lamports_account.clone(),
        ],
        &[&[
            &accounts.market.key.to_bytes(),
            &[market_state.signer_nonce],
        ]],
    )?;

    market_state.tag = AccountTag::Closed;
    let mut market_state_data: &mut [u8] = &mut accounts.market.data.borrow_mut();
    market_state.serialize(&mut market_state_data).unwrap();

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
    check_account_key(
        accounts.aaob_program,
        &market_state.aaob_program,
        DexError::InvalidAobProgramAccount,
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
