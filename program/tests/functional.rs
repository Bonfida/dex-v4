use agnostic_orderbook::state::MarketState;
use borsh::BorshDeserialize;
use dex_v3::instruction::create_market;
use dex_v3::instruction::new_order;
use solana_program::pubkey::Pubkey;
use solana_program::system_instruction::create_account;
use solana_program_test::{processor, ProgramTest};
use solana_sdk::signature::Keypair;
use solana_sdk::signature::Signer;
pub mod common;
use crate::common::utils::create_associated_token;
use crate::common::utils::{create_market_and_accounts, sign_send_instructions};

#[tokio::test]
async fn test_agnostic_orderbook() {
    // Create program and test environment
    let dex_program_id = Pubkey::new_unique();
    let aaob_program_id = Pubkey::new_unique();

    let mut program_test = ProgramTest::new(
        "agnostic_orderbook",
        aaob_program_id,
        processor!(agnostic_orderbook::entrypoint::process_instruction),
    );
    program_test.add_program(
        "dex-v3",
        dex_program_id,
        processor!(dex_v3::entrypoint::process_instruction),
    );

    // Create test context
    let mut prg_test_ctx = program_test.start_with_context().await;

    // Create the AAOB market with all accounts
    let caller_authority = Keypair::new();
    let aaob_market_account =
        create_market_and_accounts(&mut prg_test_ctx, aaob_program_id, &caller_authority).await;

    let aaob_market_state_data = prg_test_ctx
        .banks_client
        .get_account(aaob_market_account)
        .await
        .unwrap()
        .unwrap();
    let aaob_market_state =
        MarketState::deserialize(&mut &aaob_market_state_data.data[..]).unwrap();

    // Create market account
    let market_account = Keypair::new();
    let create_market_account_instruction = create_account(
        &prg_test_ctx.payer.pubkey(),
        &market_account.pubkey(),
        1_000_000,
        1_000_000,
        &dex_program_id,
    );
    sign_send_instructions(
        &mut prg_test_ctx,
        vec![create_market_account_instruction],
        vec![&market_account],
    )
    .await
    .unwrap();

    // Create the vault accounts
    let base_mint = Pubkey::new_unique();
    let quote_mint = Pubkey::new_unique();
    let base_vault =
        create_associated_token(&mut prg_test_ctx, &base_mint, &market_account.pubkey()).await;
    let quote_vault =
        create_associated_token(&mut prg_test_ctx, &quote_mint, &market_account.pubkey()).await;

    // Create the dex market
    let (market_signer, signer_nonce) =
        Pubkey::find_program_address(&[&market_account.pubkey().to_bytes()], &dex_program_id);
    let create_market_instruction = create_market(
        dex_program_id,
        market_account.pubkey(),
        aaob_market_account,
        base_vault,
        quote_vault,
        create_market::Params { signer_nonce },
    );
    sign_send_instructions(&mut prg_test_ctx, vec![create_market_instruction], vec![])
        .await
        .unwrap();

    // Create User accounts
    let user_account = Keypair::new();
    let user_account_owner = Keypair::new();
    let create_user_account_instruction = create_account(
        &prg_test_ctx.payer.pubkey(),
        &user_account.pubkey(),
        1_000_000,
        1_000_000,
        &user_account_owner.pubkey(),
    );
    sign_send_instructions(
        &mut prg_test_ctx,
        vec![create_user_account_instruction],
        vec![&user_account],
    )
    .await
    .unwrap();
    let user_base_token_account =
        create_associated_token(&mut prg_test_ctx, &base_mint, &user_account_owner.pubkey()).await;

    // New Order
    let new_order_instruction = new_order(
        dex_program_id,
        aaob_program_id,
        market_account.pubkey(),
        market_signer,
        aaob_market_account,
        aaob_market_state.event_queue,
        aaob_market_state.bids,
        aaob_market_state.asks,
        base_vault,
        quote_vault,
        user_account.pubkey(),
        user_base_token_account,
        user_account_owner.pubkey(),
        new_order::Params {
            side: agnostic_orderbook::state::Side::Ask,
            limit_price: 1000,
            max_base_qty: 1000,
            max_quote_qty: 1000,
            order_type: new_order::OrderType::ImmediateOrCancel,
            self_trade_behavior: agnostic_orderbook::state::SelfTradeBehavior::DecrementTake,
            match_limit: 10,
        },
    );
}
