//! Extract accumulated fees from the market. This is an admin instruction
use crate::{
    error::DexError,
    processor::SWEEP_AUTHORITY,
    state::DexState,
    utils::{check_account_key, check_account_owner, check_metadata_account},
};
use bonfida_utils::checks::check_token_account_owner;
use bonfida_utils::BorshSize;
use bonfida_utils::InstructionsAccount;
use borsh::BorshDeserialize;
use borsh::BorshSerialize;
use bytemuck::{Pod, Zeroable};
use mpl_token_metadata::state::{Metadata, TokenMetadataAccount};
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
    /// The DEX market
    #[cons(writable)]
    pub market: &'a T,

    /// The DEX market signer
    pub market_signer: &'a T,

    /// The market quote token vault
    #[cons(writable)]
    pub quote_vault: &'a T,

    /// The destination token account
    #[cons(writable)]
    pub destination_token_account: &'a T,

    /// The spl token program
    pub spl_token_program: &'a T,

    /// The metadata account
    pub token_metadata: &'a T,

    /// The creator token account
    #[cons(writable)]
    pub creators_token_accounts: &'a [T],
}

impl<'a, 'b: 'a> Accounts<'a, AccountInfo<'b>> {
    pub fn parse(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();

        let a = Self {
            market: next_account_info(accounts_iter)?,
            market_signer: next_account_info(accounts_iter)?,
            quote_vault: next_account_info(accounts_iter)?,
            destination_token_account: next_account_info(accounts_iter)?,
            spl_token_program: next_account_info(accounts_iter)?,
            token_metadata: next_account_info(accounts_iter)?,
            creators_token_accounts: accounts_iter.as_slice(),
        };

        check_account_key(
            a.spl_token_program,
            &spl_token::ID,
            DexError::InvalidSplTokenProgram,
        )?;

        check_account_owner(a.market, program_id, DexError::InvalidStateAccountOwner)?;

        Ok(a)
    }
}

pub(crate) fn process(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let accounts = Accounts::parse(program_id, accounts)?;

    let mut market_state = DexState::get(accounts.market)?;
    check_accounts(program_id, &market_state, &accounts)?;
    check_metadata_account(accounts.token_metadata, &market_state.base_mint)?;

    let mut no_op = true;

    if accounts.token_metadata.data_len() != 0 && market_state.accumulated_royalties != 0 {
        no_op = false;
        let metadata: Metadata = Metadata::from_account_info(accounts.token_metadata)?;
        let mut share_sum = 0;
        if let Some(creators) = metadata.data.creators {
            for (idx, creator) in creators.into_iter().enumerate() {
                share_sum += creator.share;
                let token_destination = accounts.creators_token_accounts.get(idx).unwrap();
                let amount = market_state
                    .accumulated_royalties
                    .checked_mul(creator.share as u64)
                    .ok_or(DexError::NumericalOverflow)?
                    / 100;
                market_state.accumulated_royalties = market_state
                    .accumulated_royalties
                    .checked_sub(amount)
                    .ok_or(DexError::NumericalOverflow)?;

                check_token_account_owner(token_destination, &creator.address)?;

                let transfer_instruction = spl_token::instruction::transfer(
                    &spl_token::ID,
                    accounts.quote_vault.key,
                    token_destination.key,
                    accounts.market_signer.key,
                    &[],
                    amount,
                )?;
                invoke_signed(
                    &transfer_instruction,
                    &[
                        accounts.spl_token_program.clone(),
                        accounts.quote_vault.clone(),
                        token_destination.clone(),
                        accounts.market_signer.clone(),
                    ],
                    &[&[
                        &accounts.market.key.to_bytes(),
                        &[market_state.signer_nonce as u8],
                    ]],
                )?;
            }

            if share_sum != 100 {
                msg!("Invalid metadata shares - received {}", share_sum);
                return Err(ProgramError::InvalidAccountData);
            }
        }
    }

    if market_state.accumulated_fees != 0 {
        no_op = false;
        let transfer_instruction = spl_token::instruction::transfer(
            &spl_token::ID,
            accounts.quote_vault.key,
            accounts.destination_token_account.key,
            accounts.market_signer.key,
            &[],
            market_state.accumulated_fees,
        )?;

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
                &[market_state.signer_nonce as u8],
            ]],
        )?;

        market_state.accumulated_fees = 0;
    }

    if no_op {
        msg!("There are no fees to be extracted from this market!");
        return Err(DexError::NoOp.into());
    }

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
        accounts.quote_vault,
        &market_state.quote_vault,
        DexError::InvalidQuoteVaultAccount,
    )?;

    check_token_account_owner(accounts.destination_token_account, &SWEEP_AUTHORITY)?;

    Ok(())
}
