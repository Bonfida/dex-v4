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

// TODO: cleanup
impl PrintProgramError for DexError {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + PrintProgramError + FromPrimitive,
    {
        match self {
            DexError::InvalidOrderIndex => todo!(),
            DexError::UserAccountFull => todo!(),
            DexError::TransactionAborted => todo!(),
            DexError::MissingUserAccount => todo!(),
            DexError::OrderNotFound => todo!(),
            DexError::NoOp => todo!(),
        }
    }
}
