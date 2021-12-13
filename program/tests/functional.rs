use agnostic_orderbook::state::MarketState;
use bytemuck::try_from_bytes;
use bytemuck::try_from_bytes_mut;
use dex_v4::fee_defaults::DEFAULT_FEE_TIER_MAKER_BPS_REBATES;
use dex_v4::fee_defaults::DEFAULT_FEE_TIER_TAKER_BPS_RATES;
use dex_v4::fee_defaults::DEFAULT_FEE_TIER_THRESHOLDS;
use dex_v4::instruction::cancel_order;
use dex_v4::instruction::consume_events;
use dex_v4::instruction::create_market;
use dex_v4::instruction::initialize_account;
use dex_v4::instruction::new_order;
use dex_v4::instruction::settle;
use dex_v4::state::UserAccountHeader;
use dex_v4::state::DEX_STATE_LEN;
use dex_v4::state::USER_ACCOUNT_HEADER_LEN;
use solana_program::pubkey::Pubkey;
use solana_program::system_instruction::create_account;
use solana_program::system_program;
use solana_program_test::ProgramTest;
use solana_sdk::signature::Keypair;
use solana_sdk::signature::Signer;
use spl_token::instruction::mint_to;
use std::convert::TryInto;
pub mod common;
use crate::common::utils::create_associated_token;
use crate::common::utils::mint_bootstrap;
use crate::common::utils::{create_aob_market_and_accounts, sign_send_instructions};

#[tokio::test]
async fn test_dex() {
    // Create program and test environment
    let dex_program_id = dex_v4::ID;

    let mut program_test = ProgramTest::new(
        "dex_v4",
        dex_program_id,
        None,
        // processor!(dex_v4::entrypoint::process_instruction),
    );

    // Create the market mints
    let base_mint_auth = Keypair::new();
    let (base_mint_key, _) = mint_bootstrap(None, 6, &mut program_test, &base_mint_auth.pubkey());
    let quote_mint_auth = Keypair::new();
    let (quote_mint_key, _) = mint_bootstrap(None, 6, &mut program_test, &quote_mint_auth.pubkey());

    // Create test context
    let mut prg_test_ctx = program_test.start_with_context().await;

    // Create market account
    let market_account = Keypair::new();
    let create_market_account_instruction = create_account(
        &prg_test_ctx.payer.pubkey(),
        &market_account.pubkey(),
        1_000_000,
        DEX_STATE_LEN as u64,
        &dex_program_id,
    );
    sign_send_instructions(
        &mut prg_test_ctx,
        vec![create_market_account_instruction],
        vec![&market_account],
    )
    .await
    .unwrap();

    // Define the market signer
    let (market_signer, signer_nonce) =
        Pubkey::find_program_address(&[&market_account.pubkey().to_bytes()], &dex_program_id);

    // Create the AAOB market with all accounts
    let aaob_accounts = create_aob_market_and_accounts(&mut prg_test_ctx, dex_program_id).await;

    // Create the vault accounts
    let base_vault = create_associated_token(&mut prg_test_ctx, &base_mint_key, &market_signer)
        .await
        .unwrap();
    let quote_vault = create_associated_token(&mut prg_test_ctx, &quote_mint_key, &market_signer)
        .await
        .unwrap();

    // Create the dex market
    let market_admin = Keypair::new();
    let create_market_instruction = create_market(
        dex_program_id,
        dex_v4::instruction::create_market::Accounts {
            base_vault: &base_vault,
            quote_vault: &quote_vault,
            market: &market_account.pubkey(),
            orderbook: &aaob_accounts.market,
            market_admin: &market_admin.pubkey(),
            event_queue: &aaob_accounts.event_queue,
            asks: &aaob_accounts.asks,
            bids: &aaob_accounts.bids,
        },
        create_market::Params {
            signer_nonce: signer_nonce as u64,
            min_base_order_size: 1000,
            tick_size: 1,
            cranker_reward: 0,
            fee_tier_thresholds: DEFAULT_FEE_TIER_THRESHOLDS,
            fee_tier_maker_bps_rebates: DEFAULT_FEE_TIER_MAKER_BPS_REBATES,
            fee_tier_taker_bps_rates: DEFAULT_FEE_TIER_TAKER_BPS_RATES,
        },
    );
    sign_send_instructions(&mut prg_test_ctx, vec![create_market_instruction], vec![])
        .await
        .unwrap();

    // Create User accounts
    let user_account_owner = Keypair::new();
    let create_user_account_owner_instruction = create_account(
        &prg_test_ctx.payer.pubkey(),
        &user_account_owner.pubkey(),
        1_000_000,
        0,
        &system_program::ID,
    );
    sign_send_instructions(
        &mut prg_test_ctx,
        vec![create_user_account_owner_instruction],
        vec![&user_account_owner],
    )
    .await
    .unwrap();
    let (user_account, _) = Pubkey::find_program_address(
        &[
            &market_account.pubkey().to_bytes(),
            &user_account_owner.pubkey().to_bytes(),
        ],
        &dex_program_id,
    );
    let create_user_account_instruction = initialize_account(
        dex_program_id,
        initialize_account::Accounts {
            system_program: &system_program::ID,
            user: &user_account,
            user_owner: &user_account_owner.pubkey(),
            fee_payer: &prg_test_ctx.payer.pubkey(),
        },
        initialize_account::Params {
            market: market_account.pubkey().to_bytes(),
            max_orders: 10,
        },
    );
    sign_send_instructions(
        &mut prg_test_ctx,
        vec![create_user_account_instruction],
        vec![&user_account_owner],
    )
    .await
    .unwrap();
    let user_base_token_account = create_associated_token(
        &mut prg_test_ctx,
        &base_mint_key,
        &user_account_owner.pubkey(),
    )
    .await
    .unwrap();
    let mint_to_instruction = mint_to(
        &spl_token::ID,
        &base_mint_key,
        &user_base_token_account,
        &base_mint_auth.pubkey(),
        &[],
        1 << 25,
    )
    .unwrap();
    sign_send_instructions(
        &mut prg_test_ctx,
        vec![mint_to_instruction],
        vec![&base_mint_auth],
    )
    .await
    .unwrap();
    let user_quote_token_account = create_associated_token(
        &mut prg_test_ctx,
        &quote_mint_key,
        &user_account_owner.pubkey(),
    )
    .await
    .unwrap();
    let mint_to_instruction = mint_to(
        &spl_token::ID,
        &quote_mint_key,
        &user_quote_token_account,
        &quote_mint_auth.pubkey(),
        &[],
        1 << 25,
    )
    .unwrap();
    sign_send_instructions(
        &mut prg_test_ctx,
        vec![mint_to_instruction],
        vec![&quote_mint_auth],
    )
    .await
    .unwrap();

    let aaob_market_state_data = prg_test_ctx
        .banks_client
        .get_account(aaob_accounts.market)
        .await
        .unwrap()
        .unwrap();
    let aaob_market_state: &MarketState =
        try_from_bytes(&aaob_market_state_data.data[..std::mem::size_of::<MarketState>()]).unwrap();

    // New Order, to be cancelled
    let new_order_instruction = new_order(
        dex_program_id,
        new_order::Accounts {
            spl_token_program: &spl_token::ID,
            system_program: &system_program::ID,
            market: &market_account.pubkey(),
            orderbook: &aaob_accounts.market,
            event_queue: &Pubkey::new(&aaob_market_state.event_queue),
            bids: &Pubkey::new(&aaob_market_state.bids),
            asks: &Pubkey::new(&aaob_market_state.asks),
            base_vault: &base_vault,
            quote_vault: &quote_vault,
            user: &user_account,
            user_token_account: &user_base_token_account,
            user_owner: &user_account_owner.pubkey(),
            discount_token_account: None,
        },
        new_order::Params {
            side: agnostic_orderbook::state::Side::Ask as u8,
            limit_price: 1000,
            max_base_qty: 100_000,
            max_quote_qty: 100_000,
            order_type: new_order::OrderType::Limit as u8,
            self_trade_behavior: agnostic_orderbook::state::SelfTradeBehavior::DecrementTake as u8,
            match_limit: 10,
            _padding: [0; 5],
        },
    );
    sign_send_instructions(
        &mut prg_test_ctx,
        vec![new_order_instruction],
        vec![&user_account_owner],
    )
    .await
    .unwrap();

    let mut user_acc_data = prg_test_ctx
        .banks_client
        .get_account(user_account)
        .await
        .unwrap()
        .unwrap()
        .data;
    let user_acc: &mut UserAccountHeader =
        try_from_bytes_mut(&mut user_acc_data[..USER_ACCOUNT_HEADER_LEN]).unwrap();
    println!("Number of orders {:?}", user_acc.number_of_orders);

    // Cancel Order
    let new_order_instruction = cancel_order(
        dex_program_id,
        cancel_order::Accounts {
            market: &market_account.pubkey(),
            orderbook: &aaob_accounts.market,
            event_queue: &Pubkey::new(&aaob_market_state.event_queue),
            bids: &Pubkey::new(&aaob_market_state.bids),
            asks: &Pubkey::new(&aaob_market_state.asks),
            user: &user_account,
            user_owner: &user_account_owner.pubkey(),
        },
        cancel_order::Params {
            order_index: 0,
            order_id: {
                let offset = USER_ACCOUNT_HEADER_LEN;
                u128::from_le_bytes(user_acc_data[offset..offset + 16].try_into().unwrap())
            },
        },
    );
    sign_send_instructions(
        &mut prg_test_ctx,
        vec![new_order_instruction],
        vec![&user_account_owner],
    )
    .await
    .unwrap();

    // New Order, to be matched
    let new_order_instruction = new_order(
        dex_program_id,
        new_order::Accounts {
            spl_token_program: &spl_token::ID,
            system_program: &system_program::ID,
            market: &market_account.pubkey(),
            orderbook: &aaob_accounts.market,
            event_queue: &Pubkey::new(&aaob_market_state.event_queue),
            bids: &Pubkey::new(&aaob_market_state.bids),
            asks: &Pubkey::new(&aaob_market_state.asks),
            base_vault: &base_vault,
            quote_vault: &quote_vault,
            user: &user_account,
            user_token_account: &user_base_token_account,
            user_owner: &user_account_owner.pubkey(),
            discount_token_account: None,
        },
        new_order::Params {
            side: agnostic_orderbook::state::Side::Ask as u8,
            limit_price: 1000,
            max_base_qty: 1100,
            max_quote_qty: 1000,
            order_type: new_order::OrderType::Limit as u8,
            self_trade_behavior: agnostic_orderbook::state::SelfTradeBehavior::DecrementTake as u8,
            match_limit: 10,
            _padding: [0; 5],
        },
    );
    sign_send_instructions(
        &mut prg_test_ctx,
        vec![new_order_instruction],
        vec![&user_account_owner],
    )
    .await
    .unwrap();

    // New Order, matching
    let new_order_instruction = new_order(
        dex_program_id,
        new_order::Accounts {
            spl_token_program: &spl_token::ID,
            system_program: &system_program::ID,
            market: &market_account.pubkey(),
            orderbook: &aaob_accounts.market,
            event_queue: &Pubkey::new(&aaob_market_state.event_queue),
            bids: &Pubkey::new(&aaob_market_state.bids),
            asks: &Pubkey::new(&aaob_market_state.asks),
            base_vault: &base_vault,
            quote_vault: &quote_vault,
            user: &user_account,
            user_token_account: &user_quote_token_account,
            user_owner: &user_account_owner.pubkey(),
            discount_token_account: None,
        },
        new_order::Params {
            side: agnostic_orderbook::state::Side::Bid as u8,
            limit_price: 1000,
            max_base_qty: 1000,
            max_quote_qty: 1000,
            order_type: new_order::OrderType::Limit as u8,
            self_trade_behavior: agnostic_orderbook::state::SelfTradeBehavior::DecrementTake as u8,
            match_limit: 10,
            _padding: [0; 5],
        },
    );
    sign_send_instructions(
        &mut prg_test_ctx,
        vec![new_order_instruction],
        vec![&user_account_owner],
    )
    .await
    .unwrap();

    let reward_target = Keypair::new();

    // Consume Events
    let consume_events_instruction = consume_events(
        dex_program_id,
        consume_events::Accounts {
            market: &market_account.pubkey(),
            orderbook: &aaob_accounts.market,
            event_queue: &Pubkey::new(&aaob_market_state.event_queue),
            reward_target: &reward_target.pubkey(),
            user_accounts: &[user_account],
        },
        consume_events::Params { max_iterations: 10 },
    );
    sign_send_instructions(&mut prg_test_ctx, vec![consume_events_instruction], vec![])
        .await
        .unwrap();

    // Settle
    let settle_instruction = settle(
        dex_program_id,
        settle::Accounts {
            spl_token_program: &spl_token::ID,
            market: &market_account.pubkey(),
            base_vault: &base_vault,
            quote_vault: &quote_vault,
            market_signer: &market_signer,
            user: &user_account,
            user_owner: &user_account_owner.pubkey(),
            destination_base_account: &user_base_token_account,
            destination_quote_account: &user_quote_token_account,
        },
        settle::Params {},
    );
    sign_send_instructions(
        &mut prg_test_ctx,
        vec![settle_instruction],
        vec![&user_account_owner],
    )
    .await
    .unwrap();
}
