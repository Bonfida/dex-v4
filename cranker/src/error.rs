use thiserror::Error;
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum CrankError {
    #[error("Encountered a connection error")]
    ConnectionError,
    #[error("The parsed market state is invalid")]
    InvalidMarketState,
}
