use crate::{
    error::DexError,
    state::UserAccount,
    utils::{check_account_owner, check_signer},
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
    program_error::ProgramError,
    pubkey::Pubkey,
};
#[derive(Clone, Copy, BorshDeserialize, BorshSerialize, BorshSize, Pod, Zeroable)]
#[repr(C)]
pub struct Params {}

#[derive(InstructionsAccount)]
pub struct Accounts<'a, T> {
    #[cons(writable)]
    user: &'a T,
    #[cons(signer)]
    user_owner: &'a T,
    #[cons(writable)]
    target_lamports_account: &'a T,
}

impl<'a, 'b: 'a> Accounts<'a, AccountInfo<'b>> {
    pub fn parse(
        _program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        let a = Self {
            user: next_account_info(accounts_iter)?,
            user_owner: next_account_info(accounts_iter)?,
            target_lamports_account: next_account_info(accounts_iter)?,
        };
        check_signer(a.user_owner).map_err(|e| {
            msg!("The user account owner should be a signer for this transaction!");
            e
        })?;
        check_account_owner(a.user, _program_id, DexError::InvalidStateAccountOwner)?;

        Ok(a)
    }

    pub fn load_user_account(&self) -> Result<UserAccount<'a>, ProgramError> {
        let user_account = UserAccount::get(self.user)?;
        if user_account.header.owner != self.user_owner.key.to_bytes() {
            msg!("Invalid user account owner provided!");
            return Err(ProgramError::InvalidArgument);
        };
        Ok(user_account)
    }
}

pub(crate) fn process(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let accounts = Accounts::parse(program_id, accounts)?;

    let user_account = accounts.load_user_account()?;

    if user_account.header.number_of_orders != 0
        || user_account.header.quote_token_free != 0
        || user_account.header.base_token_free != 0
    {
        msg!("The user account cannot be closed as it has pending orders or unsettled funds");
        return Err(DexError::UserAccountStillActive.into());
    }

    let mut lamports = accounts.user.lamports.borrow_mut();
    let mut target_lamports = accounts.target_lamports_account.lamports.borrow_mut();

    **target_lamports += **lamports;
    **lamports = 0;

    Ok(())
}
