use num_derive::FromPrimitive;
use thiserror::Error;

use solana_program::{decode_error::DecodeError, program_error::ProgramError};

pub type AoResult<T = ()> = Result<T, DexError>;

//TODO clean-up
#[derive(Clone, Debug, Error, FromPrimitive)]
pub enum DexError {
    #[error("The given order index is invalid.")]
    InvalidOrderIndex,
    #[error("The user account has reached its maximum capacity for open orders.")]
    UserAccountFull,
    #[error("The transaction has been aborted.")]
    TransactionAborted,
    #[error("A required user account is missing.")]
    MissingUserAccount,
    #[error("The specified order has not been found.")]
    OrderNotFound,
    #[error("The operation is a no-op")]
    NoOp,
}

impl From<DexError> for ProgramError {
    fn from(e: DexError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for DexError {
    fn type_of() -> &'static str {
        "AOError"
    }
}
