use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{
    error::DexError,
    state::{AccountTag, UserAccount},
    utils::{check_account_owner, check_signer},
};

#[derive(BorshDeserialize, BorshSerialize)]
/**
The required arguments for a initialize_account instruction.
*/
pub struct Params {}

struct Accounts<'a, 'b: 'a> {
    user: &'a AccountInfo<'b>,
    user_owner: &'a AccountInfo<'b>,
    target_lamports_account: &'a AccountInfo<'b>,
}

impl<'a, 'b: 'a> Accounts<'a, 'b> {
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
        check_signer(&a.user_owner).unwrap();
        check_account_owner(a.user, &_program_id).unwrap();

        Ok(a)
    }

    pub fn load_user_account(&self) -> Result<UserAccount<'b>, ProgramError> {
        let user_account =
            match AccountTag::deserialize(&mut (&self.user.data.borrow() as &[u8])).unwrap() {
                AccountTag::UserAccount => {
                    let u = UserAccount::parse(&self.user)?;
                    if &u.header.owner != self.user_owner.key {
                        msg!("Invalid user account owner provided!");
                        return Err(ProgramError::InvalidArgument);
                    }
                    u
                }
                AccountTag::Uninitialized => {
                    msg!("Invalid user account!");
                    return Err(ProgramError::InvalidArgument);
                }
                _ => return Err(ProgramError::InvalidArgument),
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
