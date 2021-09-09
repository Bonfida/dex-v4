use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction::create_account,
    system_program,
    sysvar::Sysvar,
};

use crate::{
    state::{AccountTag, Order, UserAccount, UserAccountHeader},
    utils::{check_account_key, check_account_owner, check_signer},
};

#[derive(BorshDeserialize, BorshSerialize)]
/**
The required arguments for a create_market instruction.
*/
pub struct Params {
    market: Pubkey,
    max_orders: u64,
}

#[derive(BorshDeserialize, BorshSerialize, Debug, PartialEq)]
pub enum OrderType {
    Limit,
    ImmediateOrCancel,
    FillOrKill,
    PostOnly,
}

struct Accounts<'a, 'b: 'a> {
    system_program: &'a AccountInfo<'b>,
    rent_sysvar: &'a AccountInfo<'b>,
    user: &'a AccountInfo<'b>,
    user_owner: &'a AccountInfo<'b>,
    fee_payer: &'a AccountInfo<'b>,
}

impl<'a, 'b: 'a> Accounts<'a, 'b> {
    pub fn parse(
        _program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        let a = Self {
            system_program: next_account_info(accounts_iter)?,
            rent_sysvar: next_account_info(accounts_iter)?,
            user: next_account_info(accounts_iter)?,
            user_owner: next_account_info(accounts_iter)?,
            fee_payer: next_account_info(accounts_iter)?,
        };
        check_signer(&a.user_owner).unwrap();
        check_account_key(a.system_program, &system_program::ID).unwrap();
        check_account_owner(a.user, &system_program::ID).unwrap();

        Ok(a)
    }
}

pub(crate) fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    params: Params,
) -> ProgramResult {
    let accounts = Accounts::parse(program_id, accounts)?;

    let Params { market, max_orders } = params;

    let (user_account_key, user_account_nonce) = Pubkey::find_program_address(
        &[&market.to_bytes(), &accounts.user_owner.key.to_bytes()],
        program_id,
    );

    if &user_account_key != accounts.user.key {
        msg!("Provided an invalid user account for the specified market and owner");
        return Err(ProgramError::InvalidArgument);
    }

    if max_orders == 0 {
        msg!("The minimum number of orders an account should be able to hold is 1");
        return Err(ProgramError::InvalidArgument);
    }
    let space = (UserAccountHeader::LEN as u64) + max_orders * (u128::LEN as u64);

    let lamports = Rent::from_account_info(accounts.rent_sysvar)?.minimum_balance(space as usize);

    let allocate_account = create_account(
        accounts.fee_payer.key,
        accounts.user.key,
        lamports,
        space,
        accounts.user_owner.key,
    );

    invoke_signed(
        &allocate_account,
        &[
            accounts.system_program.clone(),
            accounts.fee_payer.clone(),
            accounts.user.clone(),
        ],
        &[&[
            &market.to_bytes(),
            &accounts.user_owner.key.to_bytes(),
            &[user_account_nonce],
        ]],
    )?;

    let u = UserAccount::new(
        accounts.user,
        UserAccountHeader {
            tag: AccountTag::UserAccount,
            market,
            owner: *accounts.user_owner.key,
            base_token_free: 0,
            base_token_locked: 0,
            quote_token_free: 0,
            quote_token_locked: 0,
            number_of_orders: 0,
            accumulated_rebates: 0,
        },
    );

    u.write();

    Ok(())
}
