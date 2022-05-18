//! Extract available base and quote token assets from a user account
use crate::{
    error::DexError,
    state::{DexState, UserAccount},
    utils::{check_account_key, check_account_owner, check_signer},
};
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
    program_error::ProgramError,
    pubkey::Pubkey,
};

#[derive(Clone, Copy, BorshDeserialize, BorshSerialize, BorshSize, Pod, Zeroable)]
#[repr(C)]
pub struct Params {}

#[derive(InstructionsAccount)]
pub struct Accounts<'a, T> {
    /// The spl token program
    pub spl_token_program: &'a T,

    /// The DEX market
    pub market: &'a T,

    /// The base token vault
    #[cons(writable)]
    pub base_vault: &'a T,

    /// The quote token vault
    #[cons(writable)]
    pub quote_vault: &'a T,

    /// The DEX market signer account
    pub market_signer: &'a T,

    /// The DEX user account  
    #[cons(writable)]
    pub user: &'a T,

    /// The DEX user account owner wallet
    #[cons(signer)]
    pub user_owner: &'a T,

    /// The destination base token account
    #[cons(writable)]
    pub destination_base_account: &'a T,

    /// The destination quote token account
    #[cons(writable)]
    pub destination_quote_account: &'a T,
}

impl<'a, 'b: 'a> Accounts<'a, AccountInfo<'b>> {
    pub fn parse(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        let a = Self {
            spl_token_program: next_account_info(accounts_iter)?,
            market: next_account_info(accounts_iter)?,
            base_vault: next_account_info(accounts_iter)?,
            quote_vault: next_account_info(accounts_iter)?,
            market_signer: next_account_info(accounts_iter)?,
            user: next_account_info(accounts_iter)?,
            user_owner: next_account_info(accounts_iter)?,
            destination_base_account: next_account_info(accounts_iter)?,
            destination_quote_account: next_account_info(accounts_iter)?,
        };
        check_signer(a.user_owner).map_err(|e| {
            msg!("The user account owner should be a signer for this transaction!");
            e
        })?;
        check_account_key(
            a.spl_token_program,
            &spl_token::ID,
            DexError::InvalidSplTokenProgram,
        )?;
        check_account_owner(a.market, program_id, DexError::InvalidStateAccountOwner)?;
        check_account_owner(a.user, program_id, DexError::InvalidStateAccountOwner)?;

        Ok(a)
    }

    pub fn load_user_account(
        &self,
        user_account_data: &'a mut [u8],
    ) -> Result<UserAccount<'a>, ProgramError> {
        let user_account = UserAccount::from_buffer(user_account_data)?;
        if &user_account.header.owner != self.user_owner.key {
            msg!("Invalid user account owner provided!");
            return Err(ProgramError::InvalidArgument);
        }
        if &user_account.header.market != self.market.key {
            msg!("The provided user account doesn't match the current market");
            return Err(ProgramError::InvalidArgument);
        };
        Ok(user_account)
    }
}

pub(crate) fn process(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let accounts = Accounts::parse(program_id, accounts)?;

    let market_state = DexState::get(accounts.market)?;

    let mut user_account_data = accounts.user.data.borrow_mut();
    let mut user_account = accounts.load_user_account(&mut user_account_data)?;

    check_accounts(program_id, &market_state, &accounts).unwrap();

    let transfer_quote_instruction = spl_token::instruction::transfer(
        &spl_token::ID,
        &market_state.quote_vault,
        accounts.destination_quote_account.key,
        accounts.market_signer.key,
        &[],
        user_account.header.quote_token_free,
    )?;

    invoke_signed(
        &transfer_quote_instruction,
        &[
            accounts.spl_token_program.clone(),
            accounts.quote_vault.clone(),
            accounts.destination_quote_account.clone(),
            accounts.market_signer.clone(),
        ],
        &[&[
            &accounts.market.key.to_bytes(),
            &[market_state.signer_nonce as u8],
        ]],
    )?;

    let transfer_base_instruction = spl_token::instruction::transfer(
        &spl_token::ID,
        &market_state.base_vault,
        accounts.destination_base_account.key,
        accounts.market_signer.key,
        &[],
        user_account.header.base_token_free,
    )?;

    invoke_signed(
        &transfer_base_instruction,
        &[
            accounts.spl_token_program.clone(),
            accounts.base_vault.clone(),
            accounts.destination_base_account.clone(),
            accounts.market_signer.clone(),
        ],
        &[&[
            &accounts.market.key.to_bytes(),
            &[market_state.signer_nonce as u8],
        ]],
    )?;

    user_account.header.quote_token_free = 0;
    user_account.header.base_token_free = 0;

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
