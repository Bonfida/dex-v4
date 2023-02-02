use asset_agnostic_orderbook::state::market_state::MarketState;
use asset_agnostic_orderbook::state::AccountTag;
use bytemuck::try_from_bytes_mut;
use dex_v4::instruction_auto::cancel_order;
use dex_v4::instruction_auto::consume_events;
use dex_v4::instruction_auto::create_market;
use dex_v4::instruction_auto::initialize_account;
use dex_v4::instruction_auto::new_order;
use dex_v4::instruction_auto::settle;
use dex_v4::instruction_auto::swap;
use dex_v4::instruction_auto::sweep_fees;
use dex_v4::state::UserAccountHeader;
use dex_v4::state::DEX_STATE_LEN;
use dex_v4::state::USER_ACCOUNT_HEADER_LEN;
use mpl_token_metadata::pda::find_metadata_account;
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program_error::PrintProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::system_instruction::create_account;
use solana_program::system_program;
use solana_program_test::processor;
use solana_program_test::ProgramTest;
use solana_sdk::signature::Keypair;
use solana_sdk::signature::Signer;
use spl_token::instruction::mint_to;
use std::convert::TryInto;
pub mod common;
use crate::common::utils::create_associated_token;
use crate::common::utils::mint_bootstrap;
use crate::common::utils::{create_aob_market_and_accounts, sign_send_instructions};
use dex_v4::instruction_auto::update_royalties;
use mpl_token_metadata::state::Creator;
use solana_program::pubkey;

#[tokio::test]
async fn test_dex() {
    // Create program and test environment
    let dex_program_id = dex_v4::ID;
    let sweep_authority = pubkey!("DjXsn34uz8hnC4KLiSkEVNmzqX5ZFP2Q7aErTBH8LWxe");

    let mut program_test = ProgramTest::new(
        "dex_v4",
        dex_program_id,
        processor!(dex_v4::entrypoint::process_instruction),
    );

    program_test.add_program("mpl_token_metadata", mpl_token_metadata::ID, None);
    let user_account_owner = Keypair::new();

    // Create the market mints
    let base_mint_auth = Keypair::new();
    let (base_mint_key, _) = mint_bootstrap(None, 0, &mut program_test, &base_mint_auth.pubkey());
    let quote_mint_auth = Keypair::new();
    let (quote_mint_key, _) = mint_bootstrap(None, 6, &mut program_test, &quote_mint_auth.pubkey());

    // Create test context
    let mut prg_test_ctx = program_test.start_with_context().await;
    let rent = prg_test_ctx.banks_client.get_rent().await.unwrap();

    // Create metadata
    let (metadata_account_key, _) = find_metadata_account(&base_mint_key);
    let ix = mpl_token_metadata::instruction::create_metadata_accounts_v2(
        mpl_token_metadata::ID,
        metadata_account_key,
        base_mint_key,
        base_mint_auth.pubkey(),
        prg_test_ctx.payer.pubkey(),
        base_mint_auth.pubkey(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        Some(vec![
            Creator {
                address: user_account_owner.pubkey(),
                verified: false,
                share: 50,
            },
            Creator {
                address: base_mint_auth.pubkey(),
                verified: false,
                share: 50,
            },
        ]),
        5_000,
        true,
        false,
        None,
        None,
    );

    sign_send_instructions(&mut prg_test_ctx, vec![ix], vec![&base_mint_auth])
        .await
        .unwrap();
    let ix = mpl_token_metadata::instruction::sign_metadata(
        mpl_token_metadata::ID,
        metadata_account_key,
        user_account_owner.pubkey(),
    );
    sign_send_instructions(&mut prg_test_ctx, vec![ix], vec![&user_account_owner])
        .await
        .unwrap();

    // Create market account
    let market_rent = rent.minimum_balance(dex_v4::state::DEX_STATE_LEN);
    let market_account = Keypair::new();
    let create_market_account_instruction = create_account(
        &prg_test_ctx.payer.pubkey(),
        &market_account.pubkey(),
        market_rent,
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
        dex_v4::instruction_auto::create_market::Accounts {
            base_vault: &base_vault,
            quote_vault: &quote_vault,
            market: &market_account.pubkey(),
            orderbook: &aaob_accounts.market,
            market_admin: &market_admin.pubkey(),
            event_queue: &aaob_accounts.event_queue,
            asks: &aaob_accounts.asks,
            bids: &aaob_accounts.bids,
            token_metadata: &find_metadata_account(&base_mint_key).0,
        },
        create_market::Params {
            signer_nonce: signer_nonce as u64,
            min_base_order_size: 1,
            tick_size: 42949672,
            base_currency_multiplier: 1,
            quote_currency_multiplier: 10000,
        },
    );
    sign_send_instructions(&mut prg_test_ctx, vec![create_market_instruction], vec![])
        .await
        .unwrap();

    // close the market
    // let close_market_instruction = close_market(
    //     dex_program_id,
    //     dex_v4::instruction_auto::close_market::Accounts {
    //         base_vault: &base_vault,
    //         quote_vault: &quote_vault,
    //         market: &market_account.pubkey(),
    //         orderbook: &aaob_accounts.market,
    //         market_admin: &market_admin.pubkey(),
    //         event_queue: &aaob_accounts.event_queue,
    //         asks: &aaob_accounts.asks,
    //         bids: &aaob_accounts.bids,
    //         target_lamports_account: &Pubkey::new_unique(),
    //         market_signer: &market_signer,
    //         spl_token_program: &spl_token::ID,
    //     },
    //     close_market::Params {},
    // );
    // sign_send_instructions(
    //     &mut prg_test_ctx,
    //     vec![close_market_instruction],
    //     vec![&market_admin],
    // )
    // .await
    // .unwrap();

    // Create User accounts
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
            market: market_account.pubkey(),
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
    let base_mint_auth_token_account =
        create_associated_token(&mut prg_test_ctx, &quote_mint_key, &base_mint_auth.pubkey())
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

    // Create sweep fees token account
    let sweep_fees_ata =
        create_associated_token(&mut prg_test_ctx, &quote_mint_key, &sweep_authority)
            .await
            .unwrap();

    let mut aaob_market_state_data = prg_test_ctx
        .banks_client
        .get_account(aaob_accounts.market)
        .await
        .unwrap()
        .unwrap();
    let aaob_market_state =
        MarketState::from_buffer(&mut aaob_market_state_data.data, AccountTag::Market).unwrap();
        
    // New Order, to be cancelled
    let new_order_instruction = new_order(
        dex_program_id,
        new_order::Accounts {
            spl_token_program: &spl_token::ID,
            system_program: &system_program::ID,
            market: &market_account.pubkey(),
            orderbook: &aaob_accounts.market,
            event_queue: &aaob_market_state.event_queue,
            bids: &aaob_market_state.bids,
            asks: &aaob_market_state.asks,
            base_vault: &base_vault,
            quote_vault: &quote_vault,
            user: &user_account,
            user_token_account: &user_base_token_account,
            user_owner: &user_account_owner.pubkey(),
            discount_token_account: None,
            fee_referral_account: None,
        },
        new_order::Params {
            #[cfg(not(any(feature = "aarch64-test", target_arch = "aarch64")))]
            client_order_id: 0,
            #[cfg(any(feature = "aarch64-test", target_arch = "aarch64"))]
            client_order_id: bytemuck::cast(0u128),
            side: asset_agnostic_orderbook::state::Side::Ask as u8,
            limit_price: 9 * aaob_market_state.tick_size,
            max_base_qty: 1,
            max_quote_qty: u64::MAX,
            order_type: new_order::OrderType::Limit as u8,
            self_trade_behavior: asset_agnostic_orderbook::state::SelfTradeBehavior::DecrementTake
                as u8,
            match_limit: 10,
            has_discount_token_account: false as u8,
            _padding: 0,
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
    // let new_order_instruction = cancel_order(
    //     dex_program_id,
    //     cancel_order::Accounts {
    //         market: &market_account.pubkey(),
    //         orderbook: &aaob_accounts.market,
    //         event_queue: &aaob_market_state.event_queue,
    //         bids: &aaob_market_state.bids,
    //         asks: &aaob_market_state.asks,
    //         user: &user_account,
    //         user_owner: &user_account_owner.pubkey(),
    //     },
    //     cancel_order::Params {
    //         order_index: 0,
    //         order_id: {
    //             let offset = USER_ACCOUNT_HEADER_LEN;
    //             u128::from_le_bytes(user_acc_data[offset..offset + 16].try_into().unwrap())
    //         },
    //         is_client_id: false,
    //         _padding: [0u8; 7],
    //     },
    // );
    // sign_send_instructions(
    //     &mut prg_test_ctx,
    //     vec![new_order_instruction],
    //     vec![&user_account_owner],
    // )
    // .await
    // .unwrap();

    // New Order, to be matched, places 1000 units @ 1000 price
    let new_order_instruction = new_order(
        dex_program_id,
        new_order::Accounts {
            spl_token_program: &spl_token::ID,
            system_program: &system_program::ID,
            market: &market_account.pubkey(),
            orderbook: &aaob_accounts.market,
            event_queue: &aaob_market_state.event_queue,
            bids: &aaob_market_state.bids,
            asks: &aaob_market_state.asks,
            base_vault: &base_vault,
            quote_vault: &quote_vault,
            user: &user_account,
            user_token_account: &user_base_token_account,
            user_owner: &user_account_owner.pubkey(),
            discount_token_account: None,
            fee_referral_account: None,
        },
        new_order::Params {
            #[cfg(not(any(feature = "aarch64-test", target_arch = "aarch64")))]
            client_order_id: 0,
            #[cfg(any(feature = "aarch64-test", target_arch = "aarch64"))]
            client_order_id: bytemuck::cast(0u128),
            side: asset_agnostic_orderbook::state::Side::Bid as u8,
            limit_price: 11 * aaob_market_state.tick_size,
            max_base_qty: 1,
            max_quote_qty: u64::MAX,
            order_type: new_order::OrderType::Limit as u8,
            self_trade_behavior: asset_agnostic_orderbook::state::SelfTradeBehavior::DecrementTake
                as u8,
            match_limit: 10,
            has_discount_token_account: false as u8,
            _padding: 0,
        },
    );
    sign_send_instructions(
        &mut prg_test_ctx,
        vec![new_order_instruction],
        vec![&user_account_owner],
    )
    .await
    .unwrap();

    // New Order, matching, takes 100 units @ 1000 price
    // let new_order_instruction = new_order(
    //     dex_program_id,
    //     new_order::Accounts {
    //         spl_token_program: &spl_token::ID,
    //         system_program: &system_program::ID,
    //         market: &market_account.pubkey(),
    //         orderbook: &aaob_accounts.market,
    //         event_queue: &aaob_market_state.event_queue,
    //         bids: &aaob_market_state.bids,
    //         asks: &aaob_market_state.asks,
    //         base_vault: &base_vault,
    //         quote_vault: &quote_vault,
    //         user: &user_account,
    //         user_token_account: &user_quote_token_account,
    //         user_owner: &user_account_owner.pubkey(),
    //         discount_token_account: None,
    //         fee_referral_account: None,
    //     },
    //     new_order::Params {
    //         #[cfg(not(any(feature = "aarch64-test", target_arch = "aarch64")))]
    //         client_order_id: 0,
    //         #[cfg(any(feature = "aarch64-test", target_arch = "aarch64"))]
    //         client_order_id: bytemuck::cast(0u128),
    //         side: asset_agnostic_orderbook::state::Side::Bid as u8,
    //         limit_price: 10 * aaob_market_state.tick_size,
    //         max_base_qty: 1,
    //         max_quote_qty: u64::MAX,
    //         order_type: new_order::OrderType::ImmediateOrCancel as u8,
    //         self_trade_behavior: asset_agnostic_orderbook::state::SelfTradeBehavior::DecrementTake
    //             as u8,
    //         match_limit: 10,
    //         has_discount_token_account: false as u8,
    //         _padding: 0,
    //     },
    // );
    // sign_send_instructions(
    //     &mut prg_test_ctx,
    //     vec![new_order_instruction],
    //     vec![&user_account_owner],
    // )
    // .await
    // .unwrap();

    let reward_target = Keypair::new();

    // Consume Events
    // let consume_events_instruction = consume_events(
    //     dex_program_id,
    //     consume_events::Accounts {
    //         market: &market_account.pubkey(),
    //         orderbook: &aaob_accounts.market,
    //         event_queue: &aaob_market_state.event_queue,
    //         reward_target: &reward_target.pubkey(),
    //         user_accounts: &[user_account],
    //     },
    //     consume_events::Params {
    //         max_iterations: 10,
    //         no_op_err: 1,
    //     },
    // );
    // sign_send_instructions(&mut prg_test_ctx, vec![consume_events_instruction], vec![])
    //     .await
    //     .unwrap();

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

    // Swap, matching, takes 10 units @ 1000 price
    // let new_order_instruction = swap(
    //     dex_program_id,
    //     swap::Accounts {
    //         spl_token_program: &spl_token::ID,
    //         system_program: &system_program::ID,
    //         market: &market_account.pubkey(),
    //         orderbook: &aaob_accounts.market,
    //         event_queue: &aaob_market_state.event_queue,
    //         bids: &aaob_market_state.bids,
    //         asks: &aaob_market_state.asks,
    //         base_vault: &base_vault,
    //         quote_vault: &quote_vault,
    //         market_signer: &market_signer,
    //         user_base_account: &user_base_token_account,
    //         user_quote_account: &user_quote_token_account,
    //         user_owner: &user_account_owner.pubkey(),
    //         discount_token_account: None,
    //         fee_referral_account: None,
    //     },
    //     swap::Params {
    //         side: asset_agnostic_orderbook::state::Side::Bid as u8,
    //         base_qty: 10_000,
    //         quote_qty: 100000,
    //         match_limit: 10,
    //         has_discount_token_account: 0,
    //         _padding: [0; 6],
    //     },
    // );
    // sign_send_instructions(
    //     &mut prg_test_ctx,
    //     vec![new_order_instruction],
    //     vec![&user_account_owner],
    // )
    // .await
    // .unwrap();

    // Sweep fees
    let ix = sweep_fees(
        dex_program_id,
        sweep_fees::Accounts {
            market: &market_account.pubkey(),
            market_signer: &market_signer,
            quote_vault: &quote_vault,
            destination_token_account: &sweep_fees_ata,
            spl_token_program: &spl_token::ID,
            token_metadata: &find_metadata_account(&base_mint_key).0,
            creators_token_accounts: &[user_quote_token_account, base_mint_auth_token_account],
        },
        sweep_fees::Params {},
    );
    sign_send_instructions(&mut prg_test_ctx, vec![ix], vec![])
        .await
        .unwrap();

    // Consume Events
    let consume_events_instruction = consume_events(
        dex_program_id,
        consume_events::Accounts {
            market: &market_account.pubkey(),
            orderbook: &aaob_accounts.market,
            event_queue: &aaob_market_state.event_queue,
            reward_target: &reward_target.pubkey(),
            user_accounts: &[user_account],
        },
        consume_events::Params {
            max_iterations: 11,
            no_op_err: 1,
        },
    );
    sign_send_instructions(&mut prg_test_ctx, vec![consume_events_instruction], vec![])
        .await
        .unwrap();

    // Change royalties_bps
    let ix = update_royalties(
        dex_program_id,
        update_royalties::Accounts {
            market: &market_account.pubkey(),
            event_queue: &aaob_accounts.event_queue,
            token_metadata: &find_metadata_account(&base_mint_key).0,
            orderbook: &aaob_accounts.market,
        },
        update_royalties::Params {},
    );
    sign_send_instructions(&mut prg_test_ctx, vec![ix], vec![])
        .await
        .unwrap();

    // consume_events should not error when no_op_err == 0
    let consume_events_instruction = consume_events(
        dex_program_id,
        consume_events::Accounts {
            market: &market_account.pubkey(),
            orderbook: &aaob_accounts.market,
            event_queue: &aaob_market_state.event_queue,
            reward_target: &reward_target.pubkey(),
            user_accounts: &[user_account],
        },
        consume_events::Params {
            max_iterations: 10,
            no_op_err: 0,
        },
    );
    sign_send_instructions(&mut prg_test_ctx, vec![consume_events_instruction], vec![])
        .await
        .unwrap();
}
