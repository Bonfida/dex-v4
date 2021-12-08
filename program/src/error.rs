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
    #[error("The user does not own enough lamports")]
    OutofFunds,
    #[error("The user account is still active")]
    UserAccountStillActive,
    #[error("Market is still active")]
    MarketStillActive,
    #[error("Invalid market signer provided")]
    InvalidMarketSignerAccount,
    #[error("Invalid orderbook account provided")]
    InvalidOrderbookAccount,
    #[error("Invalid AOB program account provided")]
    InvalidAobProgramAccount,
    #[error("Invalid market admin account provided")]
    InvalidMarketAdminAccount,
    #[error("Invalid base vault account provided")]
    InvalidBaseVaultAccount,
    #[error("Invalid quote vault account provided")]
    InvalidQuoteVaultAccount,
    #[error("Invalid system program account provided")]
    InvalidSystemProgramAccount,
    #[error("Invalid spl token program account provided")]
    InvalidSplTokenProgram,
    #[error("A provided state account was not owned by the current program")]
    InvalidStateAccountOwner,
    #[error("The AOB instruction call returned an error")]
    AOBError,
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
