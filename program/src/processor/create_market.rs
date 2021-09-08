use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::{self, Clock},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    sysvar::{Sysvar, SysvarId},
};

use crate::{
    state::{AccountTag, DexState},
    utils::check_account_key,
};

#[derive(BorshDeserialize, BorshSerialize)]
/**
The required arguments for a create_market instruction.
*/
pub struct Params {
    pub signer_nonce: u8,
}

struct Accounts<'a, 'b: 'a> {
    sysvar_clock: &'a AccountInfo<'b>,
    market: &'a AccountInfo<'b>,
    orderbook: &'a AccountInfo<'b>,
    base_vault: &'a AccountInfo<'b>,
    quote_vault: &'a AccountInfo<'b>,
    aaob_program: &'a AccountInfo<'b>,
}

impl<'a, 'b: 'a> Accounts<'a, 'b> {
    pub fn parse(
        _program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();

        let a = Self {
            sysvar_clock: next_account_info(accounts_iter)?,
            market: next_account_info(accounts_iter)?,
            orderbook: next_account_info(accounts_iter)?,
            base_vault: next_account_info(accounts_iter)?,
            quote_vault: next_account_info(accounts_iter)?,
            aaob_program: next_account_info(accounts_iter)?,
        };

        check_account_key(a.sysvar_clock, &clock::Clock::id()).unwrap();

        Ok(a)
    }
}

pub(crate) fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    params: Params,
) -> ProgramResult {
    let accounts = Accounts::parse(program_id, accounts)?;

    let Params { signer_nonce } = params;
    let market_signer = Pubkey::create_program_address(
        &[&accounts.market.key.to_bytes(), &[signer_nonce]],
        program_id,
    )?;

    let base_mint = check_vault_account_and_get_mint(accounts.base_vault, &market_signer)?;
    let quote_mint = check_vault_account_and_get_mint(accounts.quote_vault, &market_signer)?;
    check_orderbook(&accounts.orderbook, &market_signer)?;
    //TODO check ownership of accounts

    //TODO create the aaob market?

    let current_timestamp = Clock::from_account_info(accounts.sysvar_clock)?.unix_timestamp;

    let market_state = DexState {
        tag: AccountTag::DexState,
        signer_nonce,
        base_mint,
        quote_mint,
        base_vault: *accounts.base_vault.key,
        quote_vault: *accounts.quote_vault.key,
        orderbook: *accounts.orderbook.key,
        aaob_program: *accounts.aaob_program.key,
        creation_timestamp: current_timestamp,
    };

    let mut market_data: &mut [u8] = &mut accounts.market.data.borrow_mut();
    market_state.serialize(&mut market_data).unwrap();

    Ok(())
}

fn check_vault_account_and_get_mint(
    account: &AccountInfo,
    market_signer: &Pubkey,
) -> Result<Pubkey, ProgramError> {
    let acc = spl_token::state::Account::unpack(&account.data.borrow())?;
    if &acc.owner != market_signer {
        msg!("The vault accoutn should be owned by the market signer");
        return Err(ProgramError::InvalidArgument);
    }
    if acc.close_authority.is_some() || acc.delegate.is_some() {
        msg!("Invalid vault account provided");
        return Err(ProgramError::InvalidArgument);
    }
    Ok(acc.mint)
}

fn check_orderbook(account: &AccountInfo, market_signer: &Pubkey) -> ProgramResult {
    let orderbook_state = agnostic_orderbook::state::MarketState::deserialize(
        &mut (&account.data.borrow() as &[u8]),
    )?;
    if orderbook_state.tag != agnostic_orderbook::state::AccountTag::Market {
        msg!("Invalid orderbook");
        return Err(ProgramError::InvalidArgument);
    }
    if &orderbook_state.caller_authority != market_signer {
        msg!("The provided orderbook isn't owned by the market signer.");
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}
