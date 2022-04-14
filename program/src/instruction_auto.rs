#![allow(clippy::too_many_arguments)]
use crate::processor::close_account;
pub use crate::processor::{
    cancel_order, close_market, consume_events, create_market, initialize_account, new_order,
    settle, swap, sweep_fees,
};
use bonfida_utils::InstructionsAccount;
use num_derive::{FromPrimitive, ToPrimitive};
use solana_program::{instruction::Instruction, pubkey::Pubkey};
#[derive(Clone, Copy, FromPrimitive, ToPrimitive)]
///    Describes all possible instructions and their required accounts
pub enum DexInstruction {
    /// Creates a new DEX market
    ///
    /// | Index | Writable | Signer | Description                 |
    /// | ------------------------------------------------------- |
    /// | 0     | ✅        | ❌      | The market account          |
    /// | 1     | ✅        | ❌      | The orderbook account       |
    /// | 2     | ❌        | ❌      | The base vault account      |
    /// | 3     | ❌        | ❌      | The quote vault account     |
    /// | 4     | ❌        | ❌      | The market admin account    |
    /// | 5     | ✅        | ❌      | The AOB event queue account |
    /// | 6     | ✅        | ❌      | The AOB asks account        |
    /// | 7     | ✅        | ❌      | The AOB bids account        |
    CreateMarket,
    /// Execute a new order instruction. Supported types include Limit, IOC, FOK, or Post only.
    ///
    /// | Index | Writable | Signer | Description                                                                        |
    /// | -------------------------------------------------------------------------------------------------------------- |
    /// | 0     | ❌        | ❌      | The SPL token program                                                              |
    /// | 1     | ❌        | ❌      | The system program                                                                 |
    /// | 2     | ✅        | ❌      | The DEX market                                                                     |
    /// | 3     | ✅        | ❌      | The orderbook                                                                      |
    /// | 4     | ✅        | ❌      | The AOB event queue                                                                |
    /// | 5     | ✅        | ❌      | The AOB bids shared memory                                                         |
    /// | 6     | ✅        | ❌      | The AOB asks shared memory                                                         |
    /// | 7     | ✅        | ❌      | The base token vault                                                               |
    /// | 8     | ✅        | ❌      | The quote token vault                                                              |
    /// | 9     | ✅        | ❌      | The DEX user account                                                               |
    /// | 10    | ✅        | ❌      | The user source token account                                                      |
    /// | 11    | ✅        | ✅      | The user wallet                                                                    |
    /// | 12    | ❌        | ❌      | The optional SRM or MSRM discount token account (must be owned by the user wallet) |
    /// | 13    | ✅        | ❌      | The optional referrer's token account which will receive a 20% cut of the fees     |
    NewOrder,
    ///
    /// | Index | Writable | Signer | Description                                                                        |
    /// | -------------------------------------------------------------------------------------------------------------- |
    /// | 0     | ❌        | ❌      | The SPL token program                                                              |
    /// | 1     | ❌        | ❌      | The system program                                                                 |
    /// | 2     | ✅        | ❌      | The DEX market                                                                     |
    /// | 3     | ✅        | ❌      | The orderbook                                                                      |
    /// | 4     | ✅        | ❌      | The AOB event queue                                                                |
    /// | 5     | ✅        | ❌      | The AOB bids shared memory                                                         |
    /// | 6     | ✅        | ❌      | The AOB asks shared memory                                                         |
    /// | 7     | ✅        | ❌      | The base token vault                                                               |
    /// | 8     | ✅        | ❌      | The quote token vault                                                              |
    /// | 9     | ❌        | ❌      | The DEX market signer                                                              |
    /// | 10    | ✅        | ❌      | The user base token account                                                        |
    /// | 11    | ✅        | ❌      | The user quote token account                                                       |
    /// | 12    | ✅        | ✅      | The user wallet                                                                    |
    /// | 13    | ❌        | ❌      | The optional SRM or MSRM discount token account (must be owned by the user wallet) |
    /// | 14    | ✅        | ❌      | The optional referrer's token account which will receive a 20% cut of the fees     |
    Swap,
    /// Cancel an existing order and remove it from the orderbook.
    ///
    /// | Index | Writable | Signer | Description                |
    /// | ------------------------------------------------------ |
    /// | 0     | ❌        | ❌      | The DEX market             |
    /// | 1     | ✅        | ❌      | The orderbook              |
    /// | 2     | ✅        | ❌      | The AOB event queue        |
    /// | 3     | ✅        | ❌      | The AOB bids shared memory |
    /// | 4     | ✅        | ❌      | The AOB asks shared memory |
    /// | 5     | ✅        | ❌      | The DEX user account       |
    /// | 6     | ❌        | ✅      | The user wallet            |
    CancelOrder,
    /// Crank the processing of DEX events.
    ///
    /// | Index    | Writable | Signer | Description                |
    /// | --------------------------------------------------------- |
    /// | 0        | ✅        | ❌      | The DEX market             |
    /// | 1        | ✅        | ❌      | The orderbook              |
    /// | 2        | ✅        | ❌      | The AOB event queue        |
    /// | 3        | ✅        | ❌      | The reward target          |
    /// | 4..4 + N | ✅        | ❌      | The relevant user accounts |
    ConsumeEvents,
    /// Extract available base and quote token assets from a user account
    ///
    /// | Index | Writable | Signer | Description                         |
    /// | --------------------------------------------------------------- |
    /// | 0     | ❌        | ❌      | The spl token program               |
    /// | 1     | ❌        | ❌      | The DEX market                      |
    /// | 2     | ✅        | ❌      | The base token vault                |
    /// | 3     | ✅        | ❌      | The quote token vault               |
    /// | 4     | ❌        | ❌      | The DEX market signer account       |
    /// | 5     | ✅        | ❌      | The DEX user account                |
    /// | 6     | ❌        | ✅      | The DEX user account owner wallet   |
    /// | 7     | ✅        | ❌      | The destination base token account  |
    /// | 8     | ✅        | ❌      | The destination quote token account |
    Settle,
    /// Initialize a new user account
    ///
    /// | Index | Writable | Signer | Description                    |
    /// | ---------------------------------------------------------- |
    /// | 0     | ❌        | ❌      | The system program             |
    /// | 1     | ✅        | ❌      | The user account to initialize |
    /// | 2     | ❌        | ✅      | The owner of the user account  |
    /// | 3     | ✅        | ✅      | The fee payer                  |
    InitializeAccount,
    /// Extract accumulated fees from the market. This is an admin instruction
    ///
    /// | Index | Writable | Signer | Description                             |
    /// | ------------------------------------------------------------------- |
    /// | 0     | ✅        | ❌      | The DEX market                          |
    /// | 1     | ❌        | ❌      | The DEX market signer                   |
    /// | 2     | ❌        | ✅      | The sweep authority for the DEX program |
    /// | 3     | ✅        | ❌      | The market quote token vault            |
    /// | 4     | ✅        | ❌      | The destination token account           |
    /// | 5     | ❌        | ❌      | The spl token program                   |
    SweepFees,
    /// Close an inactive and empty user account
    ///
    /// | Index | Writable | Signer | Description                            |
    /// | ------------------------------------------------------------------ |
    /// | 0     | ✅        | ❌      | The user account to close              |
    /// | 1     | ❌        | ✅      | The owner of the user account to close |
    /// | 2     | ✅        | ❌      | The target lamports account            |
    CloseAccount,
    /// Close an existing market
    ///
    /// | Index | Writable | Signer | Description                    |
    /// | ---------------------------------------------------------- |
    /// | 0     | ✅        | ❌      | The market account             |
    /// | 1     | ✅        | ❌      | The market base vault account  |
    /// | 2     | ✅        | ❌      | The market quote vault account |
    /// | 3     | ✅        | ❌      | The AOB orderbook account      |
    /// | 4     | ✅        | ❌      | The AOB event queue account    |
    /// | 5     | ✅        | ❌      | The AOB bids account           |
    /// | 6     | ✅        | ❌      | The AOB asks account           |
    /// | 7     | ❌        | ✅      | The makret admin account       |
    /// | 8     | ✅        | ❌      | The target lamports account    |
    CloseMarket,
}
///    Create a new DEX market
///   
///    The asset agnostic orderbook must be properly initialized beforehand.
pub fn create_market(
    program_id: Pubkey,
    accounts: create_market::Accounts<Pubkey>,
    params: create_market::Params,
) -> Instruction {
    accounts.get_instruction_cast(program_id, DexInstruction::CreateMarket as u8, params)
}
///   \\\\nExecute a new order on the orderbook.\\\\n\\\\nDepending on the provided parameters, the program will attempt to match the order with existing entries\\\\nin the orderbook, and then optionally post the remaining order.\\\\n
pub fn new_order(
    program_id: Pubkey,
    accounts: new_order::Accounts<Pubkey>,
    params: new_order::Params,
) -> Instruction {
    accounts.get_instruction_cast(program_id, DexInstruction::NewOrder as u8, params)
}
///    Execute a swap on the orderbook.
pub fn swap(
    program_id: Pubkey,
    accounts: swap::Accounts<Pubkey>,
    params: swap::Params,
) -> Instruction {
    accounts.get_instruction_cast(program_id, DexInstruction::Swap as u8, params)
}
///    Cancel an existing order and remove it from the orderbook.
pub fn cancel_order(
    program_id: Pubkey,
    accounts: cancel_order::Accounts<Pubkey>,
    params: cancel_order::Params,
) -> Instruction {
    accounts.get_instruction_cast(program_id, DexInstruction::CancelOrder as u8, params)
}
///    Crank the processing of DEX events.
pub fn consume_events(
    program_id: Pubkey,
    accounts: consume_events::Accounts<Pubkey>,
    params: consume_events::Params,
) -> Instruction {
    accounts.get_instruction_cast(program_id, DexInstruction::ConsumeEvents as u8, params)
}
///    Extract available base and quote token assets from a user account
pub fn settle(
    program_id: Pubkey,
    accounts: settle::Accounts<Pubkey>,
    params: settle::Params,
) -> Instruction {
    accounts.get_instruction_cast(program_id, DexInstruction::Settle as u8, params)
}
///    Initialize a new user account
pub fn initialize_account(
    program_id: Pubkey,
    accounts: initialize_account::Accounts<Pubkey>,
    params: initialize_account::Params,
) -> Instruction {
    accounts.get_instruction_cast(program_id, DexInstruction::InitializeAccount as u8, params)
}
///    Extract accumulated fees from the market. This is an admin instruction
pub fn sweep_fees(
    program_id: Pubkey,
    accounts: sweep_fees::Accounts<Pubkey>,
    params: sweep_fees::Params,
) -> Instruction {
    accounts.get_instruction_cast(program_id, DexInstruction::SweepFees as u8, params)
}
///    Close an inactive and fully settled account
pub fn close_account(
    program_id: Pubkey,
    accounts: close_account::Accounts<Pubkey>,
    params: close_account::Params,
) -> Instruction {
    accounts.get_instruction_cast(program_id, DexInstruction::CloseAccount as u8, params)
}
///    Close an existing market
pub fn close_market(
    program_id: Pubkey,
    accounts: close_market::Accounts<Pubkey>,
    params: close_market::Params,
) -> Instruction {
    accounts.get_instruction_cast(program_id, DexInstruction::CloseMarket as u8, params)
}
