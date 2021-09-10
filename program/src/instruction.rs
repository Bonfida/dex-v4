use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program, sysvar,
};

pub use crate::processor::{cancel_order, create_market, new_order};
use crate::processor::{consume_events, initialize_account, settle};
#[derive(BorshDeserialize, BorshSerialize)]
pub enum DexInstruction {
    CreateMarket(create_market::Params),
    NewOrder(new_order::Params),
    CancelOrder(cancel_order::Params),
    ConsumeEvents(consume_events::Params),
    Settle(settle::Params),
    InitializeAccount(initialize_account::Params),
}
pub fn create_market(
    dex_program_id: Pubkey,
    market_account: Pubkey,
    orderbook: Pubkey,
    base_vault: Pubkey,
    quote_vault: Pubkey,
    aaob_program: Pubkey,
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
    new_order_params: new_order::Params,
) -> Instruction {
    let data = DexInstruction::NewOrder(new_order_params)
        .try_to_vec()
        .unwrap();
    let accounts = vec![
        AccountMeta::new_readonly(agnostic_orderbook_program_id, false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(market_account, false),
        AccountMeta::new_readonly(market_signer, false),
        AccountMeta::new(orderbook, false),
        AccountMeta::new(event_queue, false),
        AccountMeta::new(bids, false),
        AccountMeta::new(asks, false),
        AccountMeta::new(base_vault, false),
        AccountMeta::new(quote_vault, false),
        AccountMeta::new(user_account, false),
        AccountMeta::new(user_token_account, false),
        AccountMeta::new_readonly(user_account_owner, true),
    ];

    Instruction {
        program_id: dex_program_id,
        accounts,
        data,
    }
}

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

#[allow(clippy::too_many_arguments)]
pub fn consume_events(
    dex_program_id: Pubkey,
    agnostic_orderbook_program_id: Pubkey,
    market_account: Pubkey,
    market_signer: Pubkey,
    orderbook: Pubkey,
    event_queue: Pubkey,
    reward_target: Pubkey,
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
    ];

    accounts.extend(user_accounts.iter().map(|k| AccountMeta::new(*k, false)));

    Instruction {
        program_id: dex_program_id,
        accounts,
        data,
    }
}

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
