use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{
    error::DexError,
    state::DexState,
    utils::{check_account_key, check_signer},
};

#[derive(BorshDeserialize, BorshSerialize)]
/**
The required arguments for a create_market instruction.
*/
pub struct Params {}

struct Accounts<'a, 'b: 'a> {
    market: &'a AccountInfo<'b>,
    market_signer: &'a AccountInfo<'b>,
    market_admin: &'a AccountInfo<'b>,
    quote_vault: &'a AccountInfo<'b>,
    destination_token_account: &'a AccountInfo<'b>,
    spl_token_program: &'a AccountInfo<'b>,
}

impl<'a, 'b: 'a> Accounts<'a, 'b> {
    pub fn parse(
        _program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();

        let a = Self {
            market: next_account_info(accounts_iter)?,
            market_signer: next_account_info(accounts_iter)?,
            market_admin: next_account_info(accounts_iter)?,
            quote_vault: next_account_info(accounts_iter)?,
            destination_token_account: next_account_info(accounts_iter)?,
            spl_token_program: next_account_info(accounts_iter)?,
        };

        check_signer(a.market_admin).unwrap();
        check_account_key(a.spl_token_program, &spl_token::ID).unwrap();

        Ok(a)
    }
}

pub(crate) fn process(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let accounts = Accounts::parse(program_id, accounts)?;

    let mut market_state =
        DexState::deserialize(&mut (&accounts.market.data.borrow() as &[u8]))?.check()?;
    check_accounts(program_id, &market_state, &accounts)?;

    let transfer_instruction = spl_token::instruction::transfer(
        &spl_token::ID,
        accounts.quote_vault.key,
        accounts.destination_token_account.key,
        accounts.market_signer.key,
        &[],
        market_state.accumulated_fees,
    )?;

    if market_state.accumulated_fees == 0 {
        msg!("Therer are no fees to be extracted from the market");
        return Err(DexError::NoOp.into());
    }

    invoke_signed(
        &transfer_instruction,
        &[
            accounts.spl_token_program.clone(),
            accounts.quote_vault.clone(),
            accounts.destination_token_account.clone(),
            accounts.market_signer.clone(),
        ],
        &[&[
            &accounts.market.key.to_bytes(),
            &[market_state.signer_nonce],
        ]],
    )?;

    market_state.accumulated_fees = 0;
    let mut market_data: &mut [u8] = &mut accounts.market.data.borrow_mut();
    market_state.serialize(&mut market_data).unwrap();

    Ok(())
}

fn check_accounts(
    program_id: &Pubkey,
    market_state: &DexState,
    accounts: &Accounts,
) -> ProgramResult {
    let market_signer = Pubkey::create_program_address(
        &[
            &accounts.market.key.to_bytes(),
            &[market_state.signer_nonce],
        ],
        program_id,
    )?;
    check_account_key(accounts.market_signer, &market_signer).unwrap();
    check_account_key(accounts.quote_vault, &market_state.quote_vault).unwrap();
    check_account_key(accounts.market_admin, &market_state.admin).unwrap();
    Ok(())
}
