//! Creates a new DEX market
use crate::{
    error::DexError,
    state::{AccountTag, DexState, MarketFeeType},
    utils::{check_account_owner, check_metadata_account, verify_metadata},
    CALLBACK_ID_LEN, CALLBACK_INFO_LEN,
};
use agnostic_orderbook::error::AoError;
use bonfida_utils::checks::check_rent_exempt;
use bonfida_utils::BorshSize;
use bonfida_utils::InstructionsAccount;
use borsh::BorshDeserialize;
use borsh::BorshSerialize;
use bytemuck::{try_from_bytes, Pod, Zeroable};
use mpl_token_metadata::state::Metadata;
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
    pub tick_size: u64,
    pub cranker_reward: u64,
    pub base_currency_multiplier: u64,
    pub quote_currency_multiplier: u64,
}

#[derive(InstructionsAccount)]
pub struct Accounts<'a, T> {
    /// The market account
    #[cons(writable)]
    pub market: &'a T,

    /// The orderbook account
    #[cons(writable)]
    pub orderbook: &'a T,

    /// The base vault account
    pub base_vault: &'a T,

    /// The quote vault account
    pub quote_vault: &'a T,

    /// The market admin account
    pub market_admin: &'a T,

    #[cons(writable)]
    /// The AOB event queue account
    pub event_queue: &'a T,

    /// The AOB asks account
    #[cons(writable)]
    pub asks: &'a T,

    /// The AOB bids account
    #[cons(writable)]
    pub bids: &'a T,

    /// The metaplex token metadata
    pub token_metadata: &'a T,
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
            token_metadata: next_account_info(accounts_iter)?,
        };

        check_account_owner(a.market, program_id, DexError::InvalidStateAccountOwner)?;
        check_account_owner(a.orderbook, program_id, DexError::InvalidStateAccountOwner)?;
        check_account_owner(
            a.base_vault,
            &spl_token::ID,
            DexError::InvalidStateAccountOwner,
        )?;
        check_account_owner(
            a.quote_vault,
            &spl_token::ID,
            DexError::InvalidStateAccountOwner,
        )?;

        Ok(a)
    }
}

pub(crate) fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let accounts = Accounts::parse(program_id, accounts)?;

    check_rent(&accounts)?;

    let Params {
        signer_nonce,
        min_base_order_size,
        tick_size,
        cranker_reward,
        base_currency_multiplier,
        quote_currency_multiplier,
    } = try_from_bytes(instruction_data).map_err(|_| ProgramError::InvalidInstructionData)?;

    if base_currency_multiplier == &0 || quote_currency_multiplier == &0 || tick_size == &0 {
        msg!("The currency multipliers and ticksize should be nonzero!");
        return Err(ProgramError::InvalidArgument);
    }

    let market_signer = Pubkey::create_program_address(
        &[&accounts.market.key.to_bytes(), &[*signer_nonce as u8]],
        program_id,
    )?;
    let base_mint = check_vault_account_and_get_mint(accounts.base_vault, &market_signer)?;
    let quote_mint = check_vault_account_and_get_mint(accounts.quote_vault, &market_signer)?;

    check_metadata_account(accounts.token_metadata, &base_mint)?;

    let current_timestamp = Clock::get()?.unix_timestamp;
    if accounts.market.data.borrow()[0] != AccountTag::Uninitialized as u8 {
        // Checking the first byte is sufficient as there is a small number of AccountTags
        msg!("The market account contains initialized state!");
        return Err(ProgramError::InvalidArgument);
    }

    let mut market_state = DexState::get_unchecked(accounts.market);

    let royalties_bps = if accounts.token_metadata.data_len() != 0 {
        let metadata = Metadata::from_account_info(accounts.token_metadata)?;
        verify_metadata(&metadata.data.creators.unwrap())?;
        metadata.data.seller_fee_basis_points
    } else {
        0
    };

    *market_state = DexState {
        tag: AccountTag::DexState as u64,
        signer_nonce: *signer_nonce as u8,
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
        min_base_order_size: *min_base_order_size / *base_currency_multiplier,
        fee_type: MarketFeeType::Default as u8,
        _padding: [0; 6],
        royalties_bps: royalties_bps as u64,
        accumulated_royalties: 0,
        base_currency_multiplier: *base_currency_multiplier,
        quote_currency_multiplier: *quote_currency_multiplier,
    };

    let invoke_params = agnostic_orderbook::instruction::create_market::Params {
        caller_authority: program_id.to_bytes(), // No impact with AOB as a lib
        callback_info_len: CALLBACK_INFO_LEN,
        callback_id_len: CALLBACK_ID_LEN,
        min_base_order_size: *min_base_order_size / *base_currency_multiplier,
        tick_size: *tick_size,
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

fn check_rent<'a>(accounts: &Accounts<'a, AccountInfo>) -> ProgramResult {
    check_rent_exempt(accounts.market)?;
    check_rent_exempt(accounts.orderbook)?;
    check_rent_exempt(accounts.base_vault)?;
    check_rent_exempt(accounts.quote_vault)?;
    check_rent_exempt(accounts.event_queue)?;
    check_rent_exempt(accounts.asks)?;
    check_rent_exempt(accounts.bids)?;
    Ok(())
}
