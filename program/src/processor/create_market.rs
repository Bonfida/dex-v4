use bytemuck::{try_from_bytes, Pod, Zeroable};
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

#[derive(Copy, Clone, Zeroable, Pod)]
#[repr(C)]
/**
The required arguments for a create_market instruction.
*/
pub struct Params {
    /// The market's signer nonce (u64 for padding)
    pub signer_nonce: u64,
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
    instruction_data: &[u8],
) -> ProgramResult {
    let accounts = Accounts::parse(program_id, accounts)?;

    let Params {
        signer_nonce,
        min_base_order_size,
    } = try_from_bytes(instruction_data).map_err(|_| ProgramError::InvalidInstructionData)?;
    let market_signer = Pubkey::create_program_address(
        &[&accounts.market.key.to_bytes(), &[*signer_nonce as u8]],
        program_id,
    )?;

    let base_mint = check_vault_account_and_get_mint(accounts.base_vault, &market_signer)?;
    let quote_mint = check_vault_account_and_get_mint(accounts.quote_vault, &market_signer)?;
    check_orderbook(&accounts.orderbook, &market_signer)?;

    let current_timestamp = Clock::get()?.unix_timestamp;

    if accounts.market.data.borrow()[0] != AccountTag::Uninitialized as u8 {
        // Checking the first byte is sufficient as there is a samll number of AccountTags
        msg!("The market account contains initialized state!");
        return Err(ProgramError::InvalidArgument);
    }

    let mut market_state = DexState::get_unchecked(accounts.market);

    *market_state = DexState {
        tag: AccountTag::DexState as u64,
        signer_nonce: *signer_nonce,
        base_mint: base_mint.to_bytes(),
        quote_mint: quote_mint.to_bytes(),
        base_vault: accounts.base_vault.key.to_bytes(),
        quote_vault: accounts.quote_vault.key.to_bytes(),
        orderbook: accounts.orderbook.key.to_bytes(),
        admin: accounts.market_admin.key.to_bytes(),
        creation_timestamp: current_timestamp,
        base_volume: 0,
        quote_volume: 0,
        accumulated_fees: 0,
        min_base_order_size: *min_base_order_size,
    };

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
    let orderbook_state = agnostic_orderbook::state::MarketState::get(account)?;
    if orderbook_state.tag != agnostic_orderbook::state::AccountTag::Market as u64 {
        msg!("Invalid orderbook");
        return Err(ProgramError::InvalidArgument);
    }
    if orderbook_state.caller_authority != market_signer.to_bytes() {
        msg!("The provided orderbook isn't owned by the market signer.");
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}
