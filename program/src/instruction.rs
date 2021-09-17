use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program, sysvar,
};

pub use crate::processor::{
    cancel_order, consume_events, create_market, initialize_account, new_order, settle, sweep_fees,
};
#[derive(BorshDeserialize, BorshSerialize)]
/// Describes all possible instructions and their required accounts
pub enum DexInstruction {
    /// Creates a new DEX market
    ///
    /// | index | writable | signer | description                                  |
    /// |-------|----------|--------|----------------------------------------------|
    /// | 0     | ❌        | ❌      | The sysvar clock account                     |
    /// | 1     | ✅        | ❌      | The market account                           |
    /// | 2     | ❌        | ❌      | The orderbook account                        |
    /// | 4     | ❌        | ❌      | The base token vault account                 |
    /// | 5     | ❌        | ❌      | The quote token vault account                |
    /// | 6     | ❌        | ❌      | The Asset Agnostic Orderbook program account |
    /// | 6     | ❌        | ❌      | The market admin account                     |
    CreateMarket(create_market::Params),
    /// Execute a new order instruction. Supported types include Limit, IOC, FOK, or Post only.
    ///
    /// | index | writable | signer | description                                                                        |
    /// |-------|----------|--------|------------------------------------------------------------------------------------|
    /// | 0     | ❌        | ❌      | The asset agnostic orderbook program                                               |
    /// | 1     | ❌        | ❌      | The SPL token program                                                              |
    /// | 3     | ❌        | ❌      | The system program                                                                 |
    /// | 4     | ❌        | ❌      | The rent sysvar                                                                    |
    /// | 5     | ✅        | ❌      | The DEX market                                                                     |
    /// | 6     | ❌        | ❌      | The DEX market signer                                                              |
    /// | 7     | ✅        | ❌      | The orderbook                                                                      |
    /// | 8     | ✅        | ❌      | The event queue                                                                    |
    /// | 9     | ✅        | ❌      | The bids shared memory                                                             |
    /// | 10    | ✅        | ❌      | The asks shared memory                                                             |
    /// | 11    | ✅        | ❌      | The base token vault                                                               |
    /// | 12    | ✅        | ❌      | The quote token vault                                                              |
    /// | 13    | ✅        | ❌      | The DEX user account                                                               |
    /// | 14    | ✅        | ❌      | The user's source token account                                                    |
    /// | 15    | ✅        | ❌      | The user's wallet                                                                  |
    /// | 16    | ✅        | ❌      | The optional SRM or MSRM discount token account (must be owned by the user wallet) |
    NewOrder(new_order::Params),
    /// Cancel an existing order and remove it from the orderbook.
    ///
    /// | index | writable | signer | description                          |
    /// |-------|----------|--------|--------------------------------------|
    /// | 0     | ❌        | ❌      | The asset agnostic orderbook program |
    /// | 1     | ❌        | ❌      | The DEX market                       |
    /// | 2     | ❌        | ❌      | The DEX market signer                |
    /// | 3     | ✅        | ❌      | The orderbook                        |
    /// | 4     | ✅        | ❌      | The event queue                      |
    /// | 5     | ✅        | ❌      | The bids shared memory               |
    /// | 6     | ✅        | ❌      | The asks shared memory               |
    /// | 7     | ✅        | ❌      | The DEX user ac count                 |
    /// | 8     | ❌        | ✅      | The user's wallet                    |
    CancelOrder(cancel_order::Params),
    /// Crank the processing of DEX events.
    ///
    /// | index | writable | signer | description                          |
    /// |-------|----------|--------|--------------------------------------|
    /// | 0     | ❌        | ❌      | The asset agnostic orderbook program |
    /// | 1     | ❌        | ❌      | The DEX market                       |
    /// | 2     | ❌        | ❌      | The DEX market signer                |
    /// | 3     | ✅        | ❌      | The orderbook                        |
    /// | 4     | ✅        | ❌      | The event queue                      |
    /// | 5     | ✅        | ❌      | The reward target                    |
    /// | 6     | ❌        | ❌      | The MSRM token account               |
    /// | 7     | ❌        | ✅      | The MSRM token account owner         |
    /// | 8..   | ✅        | ❌      | The relevant user account            |
    ConsumeEvents(consume_events::Params),
    /// Extract available base and quote token assets from a user account
    ///
    /// | index | writable | signer | description                          |
    /// |-------|----------|--------|--------------------------------------|
    /// | 0     | ❌        | ❌      | The asset agnostic orderbook program |
    /// | 1     | ❌        | ❌      | The spl token program                |
    /// | 2     | ❌        | ❌      | The DEX market                       |
    /// | 3     | ✅        | ❌      | The base token vault                 |
    /// | 4     | ✅        | ❌      | The quote token vault                |
    /// | 5     | ❌        | ❌      | The DEX market signer                |
    /// | 6     | ✅        | ❌      | The DEX user account                 |
    /// | 7     | ❌        | ✅      | The DEX user account owner wallet    |
    /// | 8     | ✅        | ❌      | The destination base token account   |
    /// | 9     | ✅        | ❌      | The destination quote token account  |
    Settle(settle::Params),
    /// Initialize a new user account
    ///
    /// | index | writable | signer | description                    |
    /// |-------|----------|--------|--------------------------------|
    /// | 0     | ❌        | ❌      | The system program             |
    /// | 1     | ❌        | ❌      | The rent sysvar                |
    /// | 2     | ✅        | ❌      | The user account to initialize |
    /// | 3     | ❌        | ✅      | The owner of the user account  |
    /// | 4     | ✅        | ✅      | The fee payer                  |
    InitializeAccount(initialize_account::Params),
    /// Extract accumulated fees from the market. This is an admin instruction
    ///
    /// | index | writable | signer | description                   |
    /// |-------|----------|--------|-------------------------------|
    /// | 0     | ✅        | ❌      | The DEX market                |
    /// | 1     | ❌        | ❌      | The market signer             |
    /// | 2     | ❌        | ✅      | The market admin              |
    /// | 3     | ✅        | ❌      | The market quote token vault  |
    /// | 4     | ✅        | ❌      | The destination token account |
    /// | 5     | ❌        | ❌      | The SPL token program         |
    SweepFees,
    /// Close an inactive and empty user account
    ///
    /// | index | writable | signer | description                            |
    /// |-------|----------|--------|----------------------------------------|
    /// | 0     | ✅        | ❌      | The user account to close              |
    /// | 1     | ❌        | ✅      | The owner of the user account to close |
    /// | 2     | ✅        | ❌      | The target lamports account            |
    CloseAccount,
}

/// Create a new DEX market
///
/// The asset agnostic orderbook must be properly initialized beforehand.
#[allow(clippy::clippy::too_many_arguments)]
pub fn create_market(
    dex_program_id: Pubkey,
    market_account: Pubkey,
    orderbook: Pubkey,
    base_vault: Pubkey,
    quote_vault: Pubkey,
    aaob_program: Pubkey,
    market_admin: Pubkey,
    create_market_params: create_market::Params,
) -> Instruction {
    let instruction_data = DexInstruction::CreateMarket(create_market_params);
    let data = instruction_data.try_to_vec().unwrap();
    let accounts = vec![
        AccountMeta::new_readonly(sysvar::clock::ID, false),
        AccountMeta::new(market_account, false),
        AccountMeta::new_readonly(orderbook, false),
        AccountMeta::new_readonly(base_vault, false),
        AccountMeta::new_readonly(quote_vault, false),
        AccountMeta::new_readonly(aaob_program, false),
        AccountMeta::new_readonly(market_admin, false),
    ];

    Instruction {
        program_id: dex_program_id,
        accounts,
        data,
    }
}
/**
Execute a new order on the orderbook.

Depending on the provided parameters, the program will attempt to match the order with existing entries
in the orderbook, and then optionally post the remaining order.
*/
#[allow(clippy::clippy::too_many_arguments)]
pub fn new_order(
    dex_program_id: Pubkey,
    agnostic_orderbook_program_id: Pubkey,
    market_account: Pubkey,
    market_signer: Pubkey,
    orderbook: Pubkey,
    event_queue: Pubkey,
    bids: Pubkey,
    asks: Pubkey,
    base_vault: Pubkey,
    quote_vault: Pubkey,
    user_account: Pubkey,
    user_token_account: Pubkey,
    user_account_owner: Pubkey,
    discount_account: Option<Pubkey>,
    new_order_params: new_order::Params,
) -> Instruction {
    let data = DexInstruction::NewOrder(new_order_params)
        .try_to_vec()
        .unwrap();
    let mut accounts = vec![
        AccountMeta::new_readonly(agnostic_orderbook_program_id, false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new(market_account, false),
        AccountMeta::new_readonly(market_signer, false),
        AccountMeta::new(orderbook, false),
        AccountMeta::new(event_queue, false),
        AccountMeta::new(bids, false),
        AccountMeta::new(asks, false),
        AccountMeta::new(base_vault, false),
        AccountMeta::new(quote_vault, false),
        AccountMeta::new(user_account, false),
        AccountMeta::new(user_token_account, false),
        AccountMeta::new(user_account_owner, true),
    ];

    if let Some(a) = discount_account {
        accounts.push(AccountMeta::new_readonly(a, false))
    }

    Instruction {
        program_id: dex_program_id,
        accounts,
        data,
    }
}

/// Cancel an existing order and remove it from the orderbook.
#[allow(clippy::clippy::too_many_arguments)]
pub fn cancel_order(
    dex_program_id: Pubkey,
    agnostic_orderbook_program_id: Pubkey,
    market_account: Pubkey,
    market_signer: Pubkey,
    orderbook: Pubkey,
    event_queue: Pubkey,
    bids: Pubkey,
    asks: Pubkey,
    user_account: Pubkey,
    user_account_owner: Pubkey,
    cancel_order_params: cancel_order::Params,
) -> Instruction {
    let data = DexInstruction::CancelOrder(cancel_order_params)
        .try_to_vec()
        .unwrap();
    let accounts = vec![
        AccountMeta::new_readonly(agnostic_orderbook_program_id, false),
        AccountMeta::new_readonly(market_account, false),
        AccountMeta::new_readonly(market_signer, false),
        AccountMeta::new(orderbook, false),
        AccountMeta::new(event_queue, false),
        AccountMeta::new(bids, false),
        AccountMeta::new(asks, false),
        AccountMeta::new(user_account, false),
        AccountMeta::new_readonly(user_account_owner, true),
    ];

    Instruction {
        program_id: dex_program_id,
        accounts,
        data,
    }
}

/// Crank the processing of DEX events.
#[allow(clippy::too_many_arguments)]
pub fn consume_events(
    dex_program_id: Pubkey,
    agnostic_orderbook_program_id: Pubkey,
    market_account: Pubkey,
    market_signer: Pubkey,
    orderbook: Pubkey,
    event_queue: Pubkey,
    reward_target: Pubkey,
    msrm_token_account: Pubkey,
    msrm_token_account_owner: Pubkey,
    user_accounts: &[Pubkey],
    consume_events_params: consume_events::Params,
) -> Instruction {
    let data = DexInstruction::ConsumeEvents(consume_events_params)
        .try_to_vec()
        .unwrap();
    let mut accounts = vec![
        AccountMeta::new_readonly(agnostic_orderbook_program_id, false),
        AccountMeta::new_readonly(market_account, false),
        AccountMeta::new_readonly(market_signer, false),
        AccountMeta::new(orderbook, false),
        AccountMeta::new(event_queue, false),
        AccountMeta::new(reward_target, false),
        AccountMeta::new_readonly(msrm_token_account, false),
        AccountMeta::new_readonly(msrm_token_account_owner, true),
    ];

    accounts.extend(user_accounts.iter().map(|k| AccountMeta::new(*k, false)));

    Instruction {
        program_id: dex_program_id,
        accounts,
        data,
    }
}

/// Initialize a new user account
#[allow(clippy::too_many_arguments)]
pub fn initialize_account(
    dex_program_id: Pubkey,
    user_account: Pubkey,
    user_account_owner: Pubkey,
    fee_payer: Pubkey,
    params: initialize_account::Params,
) -> Instruction {
    let data = DexInstruction::InitializeAccount(params)
        .try_to_vec()
        .unwrap();
    let accounts = vec![
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(sysvar::rent::ID, false),
        AccountMeta::new(user_account, false),
        AccountMeta::new_readonly(user_account_owner, true),
        AccountMeta::new(fee_payer, true),
    ];

    Instruction {
        program_id: dex_program_id,
        accounts,
        data,
    }
}

/// Extract accumulated fees from the market. This is an admin instruction
#[allow(clippy::too_many_arguments)]
pub fn sweep_fees(
    dex_program_id: Pubkey,
    market_account: Pubkey,
    market_signer: Pubkey,
    market_admin: Pubkey,
    quote_vault: Pubkey,
    destination_token_account: Pubkey,
) -> Instruction {
    let data = DexInstruction::SweepFees.try_to_vec().unwrap();
    let accounts = vec![
        AccountMeta::new(market_account, false),
        AccountMeta::new_readonly(market_signer, false),
        AccountMeta::new_readonly(market_admin, true),
        AccountMeta::new(quote_vault, false),
        AccountMeta::new(destination_token_account, false),
        AccountMeta::new_readonly(spl_token::ID, false),
    ];

    Instruction {
        program_id: dex_program_id,
        accounts,
        data,
    }
}

/// Extract available base and quote token assets from a user account
#[allow(clippy::too_many_arguments)]
pub fn settle(
    dex_program_id: Pubkey,
    agnostic_orderbook_program_id: Pubkey,
    market_account: Pubkey,
    market_signer: Pubkey,
    base_vault: Pubkey,
    quote_vault: Pubkey,
    user_account: Pubkey,
    user_account_owner: Pubkey,
    destination_base_account: Pubkey,
    destination_quote_account: Pubkey,
) -> Instruction {
    let data = DexInstruction::Settle(settle::Params {})
        .try_to_vec()
        .unwrap();
    let accounts = vec![
        AccountMeta::new_readonly(agnostic_orderbook_program_id, false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(market_account, false),
        AccountMeta::new(base_vault, false),
        AccountMeta::new(quote_vault, false),
        AccountMeta::new_readonly(market_signer, false),
        AccountMeta::new(user_account, false),
        AccountMeta::new_readonly(user_account_owner, true),
        AccountMeta::new(destination_base_account, false),
        AccountMeta::new(destination_quote_account, false),
    ];

    Instruction {
        program_id: dex_program_id,
        accounts,
        data,
    }
}

/// Close an inactive and fully settled account
pub fn close_account(
    dex_program_id: Pubkey,
    user_account: Pubkey,
    user_account_owner: Pubkey,
    target_lamports_account: Pubkey,
) -> Instruction {
    let data = DexInstruction::CloseAccount.try_to_vec().unwrap();
    let accounts = vec![
        AccountMeta::new(user_account, false),
        AccountMeta::new_readonly(user_account_owner, true),
        AccountMeta::new(target_lamports_account, false),
    ];

    Instruction {
        program_id: dex_program_id,
        accounts,
        data,
    }
}
