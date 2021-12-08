use crate::error::DexError;
use crate::processor::Processor;
use num_traits::FromPrimitive;
use solana_program::{
    account_info::AccountInfo, decode_error::DecodeError, entrypoint::ProgramResult, msg,
    program_error::PrintProgramError, pubkey::Pubkey,
};

#[cfg(not(feature = "no-entrypoint"))]
use solana_program::entrypoint;
#[cfg(not(feature = "no-entrypoint"))]
entrypoint!(process_instruction);

/// The entrypoint to the AAOB program
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    msg!("Entrypoint");
    if let Err(error) = Processor::process_instruction(program_id, accounts, instruction_data) {
        // catch the error so we can print it
        error.print::<DexError>();
        return Err(error);
    }
    Ok(())
}

impl PrintProgramError for DexError {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + PrintProgramError + FromPrimitive,
    {
        match self {
            DexError::InvalidOrderIndex => msg!("Error: The given order index is invalid."),
            DexError::UserAccountFull => {
                msg!("Error: The user account has reached its maximum capacity for open orders.")
            }
            DexError::TransactionAborted => msg!("Error: The transaction has been aborted."),
            DexError::MissingUserAccount => msg!("Error: A required user account is missing."),
            DexError::OrderNotFound => msg!("Error: The specified order has not been found."),
            DexError::NoOp => msg!("Error: The operation is a no-op"),
            DexError::OutofFunds => msg!("Error: The user does not own enough lamports"),
            DexError::UserAccountStillActive => msg!("Error: The user account is still active"),
            DexError::MarketStillActive => msg!("Error: Market is still active"),
            DexError::InvalidMarketSignerAccount => msg!("Error: Invalid market signer provided"),
            DexError::InvalidOrderbookAccount => msg!("Error: Invalid orderbook account provided"),
            DexError::InvalidAobProgramAccount => {
                msg!("Error: Invalid AOB program account provided")
            }
            DexError::InvalidMarketAdminAccount => {
                msg!("Error: Invalid market admin account provided")
            }
            DexError::InvalidBaseVaultAccount => msg!("Error: Invalid base vault account provided"),
            DexError::InvalidQuoteVaultAccount => {
                msg!("Error: Invalid quote vault account provided")
            }
            DexError::InvalidSystemProgramAccount => {
                msg!("Error: Invalid system program account provided")
            }
            DexError::InvalidSplTokenProgram => {
                msg!("Error: Invalid spl token program account provided")
            }
            DexError::InvalidStateAccountOwner => {
                msg!("Error: A provided state account was not owned by the current program")
            }
            DexError::AOBError => {
                msg!("Error: The AOB instruction call returned an error.")
            }
        }
    }
}
