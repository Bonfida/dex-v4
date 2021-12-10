use crate::{
    error::DexError,
    state::{AccountTag, DexState},
    utils::check_account_owner,
    CALLBACK_ID_LEN, CALLBACK_INFO_LEN,
};
use agnostic_orderbook::error::AoError;
use bonfida_utils::BorshSize;
use bonfida_utils::InstructionsAccount;
use borsh::BorshDeserialize;
use borsh::BorshSerialize;
use bytemuck::{try_from_bytes, Pod, Zeroable};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    program_error::{PrintProgramError, ProgramError},
    program_pack::Pack,
    pubkey::Pubkey,
    sysvar::Sysvar,
};

#[derive(Copy, Clone, Zeroable, Pod, BorshDeserialize, BorshSerialize, BorshSize)]
#[repr(C)]
/**
The required arguments for a create_market instruction.
*/
pub struct Params {
    /// The market's signer nonce (u64 for padding)
    pub signer_nonce: u64,
    /// The minimum allowed order size in base token amount
    pub min_base_order_size: u64,
    pub price_bitmask: u64,
    pub cranker_reward: u64,
    /// Fee tier thresholds
    pub fee_tier_thresholds: [u64; 6],
    /// Fee tier taker rates
    pub fee_tier_taker_bps_rates: [u64; 7],
    /// Fee tier maker rates
    pub fee_tier_maker_bps_rebates: [u64; 7],
}

#[derive(InstructionsAccount)]
pub struct Accounts<'a, T> {
    #[cons(writable)]
    pub market: &'a T,
    #[cons(writable)]
    pub orderbook: &'a T,
    pub base_vault: &'a T,
    pub quote_vault: &'a T,
    pub market_admin: &'a T,
    #[cons(writable)]
    pub event_queue: &'a T,
    #[cons(writable)]
    pub asks: &'a T,
    #[cons(writable)]
    pub bids: &'a T,
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
            base_vault: next_account_info(accounts_iter)?,
            quote_vault: next_account_info(accounts_iter)?,
            market_admin: next_account_info(accounts_iter)?,
            event_queue: next_account_info(accounts_iter)?,
            asks: next_account_info(accounts_iter)?,
            bids: next_account_info(accounts_iter)?,
        };
        check_account_owner(a.market, program_id, DexError::InvalidStateAccountOwner)?;
        check_account_owner(a.orderbook, program_id, DexError::InvalidStateAccountOwner)?;

        Ok(a)
    }
}

pub(crate) fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let accounts = Accounts::parse(program_id, accounts)?;

    msg!("{:?}", instruction_data);
    let Params {
        signer_nonce,
        min_base_order_size,
        price_bitmask,
        cranker_reward,
        fee_tier_thresholds,
        fee_tier_maker_bps_rebates: fee_tier_maker_rates,
        fee_tier_taker_bps_rates: fee_tier_taker_rates,
    } = try_from_bytes(instruction_data).map_err(|_| ProgramError::InvalidInstructionData)?;
    let market_signer = Pubkey::create_program_address(
        &[&accounts.market.key.to_bytes(), &[*signer_nonce as u8]],
        program_id,
    )?;
    let base_mint = check_vault_account_and_get_mint(accounts.base_vault, &market_signer)?;
    let quote_mint = check_vault_account_and_get_mint(accounts.quote_vault, &market_signer)?;
    let current_timestamp = Clock::get()?.unix_timestamp;
    if accounts.market.data.borrow()[0] != AccountTag::Uninitialized as u8 {
        // Checking the first byte is sufficient as there is a small number of AccountTags
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
        fee_tier_thresholds: *fee_tier_thresholds,
        fee_tier_taker_bps_rates: *fee_tier_taker_rates,
        fee_tier_maker_bps_rebates: *fee_tier_maker_rates,
    };

    let invoke_params = agnostic_orderbook::instruction::create_market::Params {
        caller_authority: *program_id, // No impact with AOB as a lib
        callback_info_len: CALLBACK_INFO_LEN,
        callback_id_len: CALLBACK_ID_LEN,
        min_base_order_size: *min_base_order_size,
        price_bitmask: *price_bitmask,
        cranker_reward: *cranker_reward,
    };
    let invoke_accounts = agnostic_orderbook::instruction::create_market::Accounts {
        market: accounts.orderbook,
        event_queue: accounts.event_queue,
        bids: accounts.bids,
        asks: accounts.asks,
    };

    if let Err(error) = agnostic_orderbook::instruction::create_market::process(
        program_id,
        invoke_accounts,
        invoke_params,
    ) {
        error.print::<AoError>();
        return Err(DexError::AOBError.into());
    }

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
