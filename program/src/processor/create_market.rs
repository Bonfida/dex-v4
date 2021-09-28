use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    sysvar::Sysvar,
};

use crate::state::{AccountTag, DexState};

#[derive(BorshDeserialize, BorshSerialize)]
/**
The required arguments for a create_market instruction.
*/
pub struct Params {
    /// The market's signer nonce
    pub signer_nonce: u8,
    /// The minimum allowed order size in base token amount
    pub min_base_order_size: u64,
}

struct Accounts<'a, 'b: 'a> {
    market: &'a AccountInfo<'b>,
    orderbook: &'a AccountInfo<'b>,
    base_vault: &'a AccountInfo<'b>,
    quote_vault: &'a AccountInfo<'b>,
    market_admin: &'a AccountInfo<'b>,
}

impl<'a, 'b: 'a> Accounts<'a, 'b> {
    pub fn parse(
        _program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();

        let a = Self {
            market: next_account_info(accounts_iter)?,
            orderbook: next_account_info(accounts_iter)?,
            base_vault: next_account_info(accounts_iter)?,
            quote_vault: next_account_info(accounts_iter)?,
            market_admin: next_account_info(accounts_iter)?,
        };

        Ok(a)
    }
}

pub(crate) fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    params: Params,
) -> ProgramResult {
    let accounts = Accounts::parse(program_id, accounts)?;

    let Params {
        signer_nonce,
        min_base_order_size,
    } = params;
    let market_signer = Pubkey::create_program_address(
        &[&accounts.market.key.to_bytes(), &[signer_nonce]],
        program_id,
    )?;

    let base_mint = check_vault_account_and_get_mint(accounts.base_vault, &market_signer)?;
    let quote_mint = check_vault_account_and_get_mint(accounts.quote_vault, &market_signer)?;
    check_orderbook(&accounts.orderbook, &market_signer)?;
    //TODO check ownership of accounts

    //TODO create the aaob market?

    let current_timestamp = Clock::get()?.unix_timestamp;

    if accounts.market.data.borrow()[0] != AccountTag::Uninitialized as u8 {
        msg!("The market account contains initialized state!");
        return Err(ProgramError::InvalidArgument);
    }

    let market_state = DexState {
        tag: AccountTag::DexState,
        signer_nonce,
        base_mint,
        quote_mint,
        base_vault: *accounts.base_vault.key,
        quote_vault: *accounts.quote_vault.key,
        orderbook: *accounts.orderbook.key,
        admin: *accounts.market_admin.key,
        creation_timestamp: current_timestamp,
        base_volume: 0,
        quote_volume: 0,
        accumulated_fees: 0,
        min_base_order_size,
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
        msg!("The vault account should be owned by the market signer");
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
