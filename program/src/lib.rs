#![warn(missing_docs)]
/*!
Orderbook-based on-chain SPL token swap market

This program is intended for use to build a decentralized exchange (DEX) specialized on SPL token swaps.
*/

#[doc(hidden)]
pub mod entrypoint;
#[doc(hidden)]
pub mod error;
/// Program instructions and their CPI-compatible bindings
pub mod instruction;
/// Describes the different data structres that the program uses to encode state
pub mod state;

pub(crate) mod processor;
pub(crate) mod utils;

pub use processor::fee_defaults;
pub use processor::{CALLBACK_ID_LEN, CALLBACK_INFO_LEN};
use solana_program::declare_id;

declare_id!("SerumSqm3PWpKcHva3sxfUPXsYaE53czAbWtgAaisCf");
