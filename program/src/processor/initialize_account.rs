use bonfida_utils::BorshSize;
use bonfida_utils::InstructionsAccount;
use borsh::BorshDeserialize;
use borsh::BorshSerialize;
use bytemuck::{try_from_bytes, Pod, Zeroable};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction::create_account,
    system_program,
    sysvar::Sysvar,
};

use crate::{
    error::DexError,
    state::{Order, UserAccount, UserAccountHeader, USER_ACCOUNT_HEADER_LEN},
    utils::{check_account_key, check_account_owner, check_signer},
};

#[derive(Clone, Copy, Zeroable, Pod, BorshDeserialize, BorshSerialize, BorshSize)]
#[repr(C)]
/**
The required arguments for a initialize_account instruction.
*/
pub struct Params {
    /// The user account's parent market
    pub market: [u8; 32],
    /// The maximum number of orders the user account may hold
    pub max_orders: u64,
}

#[derive(InstructionsAccount)]
pub struct Accounts<'a, T> {
    pub system_program: &'a T,
    #[cons(writable)]
    pub user: &'a T,
    #[cons(signer)]
    pub user_owner: &'a T,
    #[cons(writable, signer)]
    pub fee_payer: &'a T,
}

impl<'a, 'b: 'a> Accounts<'a, AccountInfo<'b>> {
    pub fn parse(
        _program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        let a = Self {
            system_program: next_account_info(accounts_iter)?,
            user: next_account_info(accounts_iter)?,
            user_owner: next_account_info(accounts_iter)?,
            fee_payer: next_account_info(accounts_iter)?,
        };
        check_signer(a.user_owner).map_err(|e| {
            msg!("The user account owner should be a signer for this transaction!");
            e
        })?;
        check_account_key(
            a.system_program,
            &system_program::ID.to_bytes(),
            DexError::InvalidSystemProgramAccount,
        )?;
        check_account_owner(
            a.user,
            &system_program::ID,
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

    let Params { market, max_orders } =
        try_from_bytes(instruction_data).map_err(|_| ProgramError::InvalidInstructionData)?;

    let (user_account_key, user_account_nonce) =
        Pubkey::find_program_address(&[market, &accounts.user_owner.key.to_bytes()], program_id);

    if &user_account_key != accounts.user.key {
        msg!("Provided an invalid user account for the specified market and owner");
        return Err(ProgramError::InvalidArgument);
    }

    if max_orders == &0 {
        msg!("The minimum number of orders an account should be able to hold is 1");
        return Err(ProgramError::InvalidArgument);
    }
    let space = (USER_ACCOUNT_HEADER_LEN as u64) + max_orders * (u128::LEN as u64);

    let lamports = Rent::get()?.minimum_balance(space as usize);

    let allocate_account = create_account(
        accounts.fee_payer.key,
        accounts.user.key,
        lamports,
        space,
        program_id,
    );

    invoke_signed(
        &allocate_account,
        &[
            accounts.system_program.clone(),
            accounts.fee_payer.clone(),
            accounts.user.clone(),
        ],
        &[&[
            market,
            &accounts.user_owner.key.to_bytes(),
            &[user_account_nonce],
        ]],
    )?;
    let mut u = UserAccount::get_unchecked(accounts.user);

    *(u.header) = UserAccountHeader::new(&Pubkey::new(market), accounts.user_owner.key);

    Ok(())
}
