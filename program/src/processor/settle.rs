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
    pub spl_token_program: &'a T,
    pub market: &'a T,
    #[cons(writable)]
    pub base_vault: &'a T,
    #[cons(writable)]
    pub quote_vault: &'a T,
    pub market_signer: &'a T,
    #[cons(writable)]
    pub user: &'a T,
    #[cons(signer)]
    pub user_owner: &'a T,
    #[cons(writable)]
    pub destination_base_account: &'a T,
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
            &spl_token::ID.to_bytes(),
            DexError::InvalidSplTokenProgram,
        )?;
        check_account_owner(a.market, program_id, DexError::InvalidStateAccountOwner)?;
        check_account_owner(a.user, program_id, DexError::InvalidStateAccountOwner)?;

        Ok(a)
    }

    pub fn load_user_account(&self) -> Result<UserAccount<'a>, ProgramError> {
        let user_account = UserAccount::get(self.user)?;
        if user_account.header.owner != self.user_owner.key.to_bytes() {
            msg!("Invalid user account owner provided!");
            return Err(ProgramError::InvalidArgument);
        }
        if user_account.header.market != self.market.key.to_bytes() {
            msg!("The provided user account doesn't match the current market");
            return Err(ProgramError::InvalidArgument);
        };
        Ok(user_account)
    }
}

pub(crate) fn process(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let accounts = Accounts::parse(program_id, accounts)?;

    let market_state = DexState::get(accounts.market)?;

    let mut user_account = accounts.load_user_account()?;

    check_accounts(program_id, &market_state, &accounts).unwrap();

    let transfer_quote_instruction = spl_token::instruction::transfer(
        &spl_token::ID,
        &Pubkey::new(&market_state.quote_vault),
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
        &Pubkey::new(&market_state.base_vault),
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
        &market_signer.to_bytes(),
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
