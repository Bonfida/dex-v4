use std::convert::TryInto;

use agnostic_orderbook::state::MarketState;
use bytemuck::try_from_bytes;
use dex_v4::fee_defaults::DEFAULT_FEE_TIER_MAKER_BPS_REBATES;
use dex_v4::fee_defaults::DEFAULT_FEE_TIER_TAKER_BPS_RATES;
use dex_v4::fee_defaults::DEFAULT_FEE_TIER_THRESHOLDS;
use dex_v4::instruction::initialize_account;
use dex_v4::instruction::new_order;
use dex_v4::state::{DexState, DEX_STATE_LEN};
use serum_dex::state::gen_vault_signer_key;
use solana_program::instruction::Instruction;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::system_instruction::create_account;
use solana_program::system_program;
use solana_program::sysvar;
use solana_program_test::{ProgramTest, ProgramTestContext};
use solana_sdk::signature::Signer;
use solana_sdk::{signature::Keypair, transport::TransportError};
use spl_token::instruction::mint_to;

use crate::common::utils::create_aob_market_and_accounts;
use crate::common::utils::create_associated_token;
use crate::common::utils::mint_bootstrap;
use crate::common::utils::sign_send_instructions;

pub(crate) const NB_USER_ACCS: u32 = 10;

pub struct AobDexTestContext {
    pub aaob_program_id: Pubkey,
    pub dex_program_id: Pubkey,
    pub dex_market_key: Pubkey,
    pub dex_market: DexState,
    pub aob_market: MarketState,
    pub user_account_keys: Vec<Pubkey>,
    // pub user_accounts: Vec<UserAccountHeader>,
    pub user_owners: Vec<Keypair>,
    pub user_bases: Vec<Pubkey>,
    pub user_quotes: Vec<Pubkey>,
}

pub struct SerumTestContext {
    serum_market: SerumMarket,
    open_orders: Vec<Pubkey>,
}

pub struct SerumMarket {
    pub market_key: Keypair,
    pub req_q_key: Keypair,
    pub event_q_key: Keypair,
    pub bids_key: Keypair,
    pub asks_key: Keypair,
    pub coin_lot_size: u64,
    pub pc_lot_size: u64,
    pub vault_signer_pk: Pubkey,
    pub vault_signer_nonce: u64,
    pub coin_vault: Pubkey,
    pub pc_vault: Pubkey,
    pub coin_mint: Pubkey,
    pub pc_mint: Pubkey,
}

/// Creates an aob dex market along with all needed accounts
/// Returns Dex market state pubkey, user account key, and test context
pub async fn create_aob_dex(
    mut program_test: ProgramTest,
    aaob_program_id: Pubkey,
    dex_program_id: Pubkey,
) -> (AobDexTestContext, ProgramTestContext) {
    // Create the market mints
    let base_mint_auth = Keypair::new();
    let (base_mint_key, _) = mint_bootstrap(None, 6, &mut program_test, &base_mint_auth.pubkey());
    let quote_mint_auth = Keypair::new();
    let (quote_mint_key, _) = mint_bootstrap(None, 6, &mut program_test, &quote_mint_auth.pubkey());

    // Create test context
    let mut pgr_test_ctx = program_test.start_with_context().await;

    // Create market account
    let market_account = Keypair::new();
    let create_market_account_instruction = create_account(
        &pgr_test_ctx.payer.pubkey(),
        &market_account.pubkey(),
        1_000_000,
        1_000_000,
        &dex_program_id,
    );
    sign_send_instructions(
        &mut pgr_test_ctx,
        vec![create_market_account_instruction],
        vec![&market_account],
    )
    .await
    .unwrap();

    // Define the market signer
    let (market_signer, signer_nonce) =
        Pubkey::find_program_address(&[&market_account.pubkey().to_bytes()], &dex_program_id);

    // Create the AAOB market with all accounts
    let aaob_accounts = create_aob_market_and_accounts(&mut pgr_test_ctx, dex_program_id).await;

    // Create the vault accounts
    let base_vault = create_associated_token(&mut pgr_test_ctx, &base_mint_key, &market_signer)
        .await
        .unwrap();
    let quote_vault = create_associated_token(&mut pgr_test_ctx, &quote_mint_key, &market_signer)
        .await
        .unwrap();

    // Create the dex market
    let market_admin = Keypair::new();
    let create_market_instruction = dex_v4::instruction::create_market(
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
        dex_v4::instruction::create_market::Params {
            signer_nonce: signer_nonce as u64,
            min_base_order_size: 1000,
            tick_size: 1,
            cranker_reward: 0,
            fee_tier_thresholds: DEFAULT_FEE_TIER_THRESHOLDS,
            fee_tier_maker_bps_rebates: DEFAULT_FEE_TIER_MAKER_BPS_REBATES,
            fee_tier_taker_bps_rates: DEFAULT_FEE_TIER_TAKER_BPS_RATES,
        },
    );
    sign_send_instructions(&mut pgr_test_ctx, vec![create_market_instruction], vec![])
        .await
        .unwrap();

    // Create User accounts
    let mut user_account_keys = Vec::with_capacity(NB_USER_ACCS as usize);
    let mut user_owners = Vec::with_capacity(NB_USER_ACCS as usize);
    let mut user_quotes = Vec::with_capacity(NB_USER_ACCS as usize);
    let mut user_bases = Vec::with_capacity(NB_USER_ACCS as usize);
    for _ in 0..NB_USER_ACCS {
        let user_account_owner = Keypair::new();
        let create_user_account_owner_instruction = create_account(
            &pgr_test_ctx.payer.pubkey(),
            &user_account_owner.pubkey(),
            1_000_000_000_000,
            0,
            &system_program::ID,
        );
        sign_send_instructions(
            &mut pgr_test_ctx,
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
                fee_payer: &pgr_test_ctx.payer.pubkey(),
            },
            initialize_account::Params {
                market: market_account.pubkey().to_bytes(),
                max_orders: 100,
            },
        );
        sign_send_instructions(
            &mut pgr_test_ctx,
            vec![create_user_account_instruction],
            vec![&user_account_owner],
        )
        .await
        .unwrap();
        user_account_keys.push(user_account);
        let user_base_token_account = create_associated_token(
            &mut pgr_test_ctx,
            &base_mint_key,
            &user_account_owner.pubkey(),
        )
        .await
        .unwrap();
        user_bases.push(user_base_token_account);
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
            &mut pgr_test_ctx,
            vec![mint_to_instruction],
            vec![&base_mint_auth],
        )
        .await
        .unwrap();

        let user_quote_token_account = create_associated_token(
            &mut pgr_test_ctx,
            &quote_mint_key,
            &user_account_owner.pubkey(),
        )
        .await
        .unwrap();
        user_quotes.push(user_quote_token_account);
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
            &mut pgr_test_ctx,
            vec![mint_to_instruction],
            vec![&quote_mint_auth],
        )
        .await
        .unwrap();
        user_owners.push(user_account_owner);
    }

    let dex_market_data = pgr_test_ctx
        .banks_client
        .get_account(market_account.pubkey())
        .await
        .unwrap()
        .unwrap()
        .data;
    let dex_market: &DexState = try_from_bytes(&dex_market_data[..DEX_STATE_LEN] as &[u8]).unwrap();
    let aob_market_data = pgr_test_ctx
        .banks_client
        .get_account(aaob_accounts.market)
        .await
        .unwrap()
        .unwrap()
        .data;
    let aob_market: MarketState = *try_from_bytes(&aob_market_data).unwrap();

    (
        AobDexTestContext {
            aaob_program_id,
            dex_program_id,
            dex_market_key: market_account.pubkey(),
            dex_market: *dex_market,
            aob_market,
            user_account_keys,
            // user_account: user_account_header,
            user_owners,
            user_bases,
            user_quotes,
        },
        pgr_test_ctx,
    )
}

pub async fn initialize_serum_market_accounts(
    pgr_test_ctx: &mut ProgramTestContext,
    aob_dex_test_ctx: &AobDexTestContext,
    serum_dex_program_id: Pubkey,
) -> Result<SerumTestContext, TransportError> {
    let (market_key, create_market) =
        create_serum_dex_account(pgr_test_ctx, serum_dex_program_id, 376)?;
    let (req_q_key, create_req_q) =
        create_serum_dex_account(pgr_test_ctx, serum_dex_program_id, 6400)?;
    let (event_q_key, create_event_q) =
        create_serum_dex_account(pgr_test_ctx, serum_dex_program_id, 1 << 20)?;
    let (bids_key, create_bids) =
        create_serum_dex_account(pgr_test_ctx, serum_dex_program_id, 1 << 16)?;
    let (asks_key, create_asks) =
        create_serum_dex_account(pgr_test_ctx, serum_dex_program_id, 1 << 16)?;
    let (vault_signer_nonce, vault_signer_pk): (u64, _) = {
        let mut i = 0;
        loop {
            assert!(i < 100);
            if let Ok(pk) = gen_vault_signer_key(i, &market_key.pubkey(), &serum_dex_program_id) {
                break (i, pk);
            }
            i += 1;
        }
    };
    let create_instructions = vec![
        create_market,
        create_req_q,
        create_event_q,
        create_bids,
        create_asks,
    ];
    let keys = vec![&market_key, &req_q_key, &event_q_key, &bids_key, &asks_key];
    sign_send_instructions(pgr_test_ctx, create_instructions, keys)
        .await
        .unwrap();

    // Create Vaults
    let coin_vault = create_associated_token(
        pgr_test_ctx,
        &Pubkey::new(&aob_dex_test_ctx.dex_market.base_mint),
        &vault_signer_pk,
    )
    .await
    .unwrap();
    let pc_vault = create_associated_token(
        pgr_test_ctx,
        &Pubkey::new(&aob_dex_test_ctx.dex_market.quote_mint),
        &vault_signer_pk,
    )
    .await
    .unwrap();

    let init_market_instruction = serum_dex::instruction::initialize_market(
        &market_key.pubkey(),
        &serum_dex_program_id,
        &Pubkey::new(&aob_dex_test_ctx.dex_market.base_mint),
        &Pubkey::new(&aob_dex_test_ctx.dex_market.quote_mint),
        &coin_vault,
        &pc_vault,
        None,
        None,
        None,
        &bids_key.pubkey(),
        &asks_key.pubkey(),
        &req_q_key.pubkey(),
        &event_q_key.pubkey(),
        1,
        1,
        vault_signer_nonce,
        100,
    )
    .unwrap();
    let serum_market = SerumMarket {
        market_key,
        req_q_key,
        event_q_key,
        bids_key,
        asks_key,
        coin_lot_size: 1,
        pc_lot_size: 1,
        vault_signer_pk,
        vault_signer_nonce,
        coin_vault,
        pc_vault,
        coin_mint: Pubkey::new(&aob_dex_test_ctx.dex_market.base_mint),
        pc_mint: Pubkey::new(&aob_dex_test_ctx.dex_market.quote_mint),
    };
    sign_send_instructions(pgr_test_ctx, vec![init_market_instruction], vec![])
        .await
        .unwrap();

    let mut open_orders = Vec::with_capacity(NB_USER_ACCS.try_into().unwrap());
    for _ in 0..NB_USER_ACCS {
        // Create user open orders account
        let (open_order, create_open_order_instruction) =
            create_serum_dex_account(pgr_test_ctx, serum_dex_program_id, 3216).unwrap();
        sign_send_instructions(
            pgr_test_ctx,
            vec![create_open_order_instruction],
            vec![&open_order],
        )
        .await
        .unwrap();
        open_orders.push(open_order.pubkey());
    }

    Ok(SerumTestContext {
        serum_market,
        open_orders,
    })
}

pub fn create_serum_dex_account(
    pgr_test_ctx: &ProgramTestContext,
    serum_program_id: Pubkey,
    unpadded_len: usize,
) -> Result<(Keypair, Instruction), TransportError> {
    let len = unpadded_len + 12;
    let key = Keypair::new();
    let create_account_instr = solana_sdk::system_instruction::create_account(
        &pgr_test_ctx.payer.pubkey(),
        &key.pubkey(),
        Rent::default().minimum_balance(len),
        len as u64,
        &serum_program_id,
    );
    Ok((key, create_account_instr))
}

#[allow(clippy::too_many_arguments)]
pub async fn aob_dex_new_order(
    pgr_test_ctx: &mut ProgramTestContext,
    dex_test_ctx: &AobDexTestContext,
    side: agnostic_orderbook::state::Side,
    limit_price: u64,
    max_base_qty: u64,
    max_quote_qty: u64,
    user_account_index: usize,
    dex_program_id: Pubkey,
) {
    // New Order on AOB DEX
    let new_order_instruction = new_order(
        dex_program_id,
        new_order::Accounts {
            spl_token_program: &spl_token::ID,
            system_program: &system_program::ID,
            market: &dex_test_ctx.dex_market_key,
            orderbook: &Pubkey::new(&dex_test_ctx.dex_market.orderbook),
            event_queue: &Pubkey::new(&dex_test_ctx.aob_market.event_queue),
            bids: &Pubkey::new(&dex_test_ctx.aob_market.bids),
            asks: &Pubkey::new(&dex_test_ctx.aob_market.asks),
            base_vault: &Pubkey::new(&dex_test_ctx.dex_market.base_vault),
            quote_vault: &Pubkey::new(&dex_test_ctx.dex_market.quote_vault),
            user: &dex_test_ctx.user_account_keys[user_account_index],
            user_token_account: &match side {
                agnostic_orderbook::state::Side::Ask => dex_test_ctx.user_bases[user_account_index],
                agnostic_orderbook::state::Side::Bid => {
                    dex_test_ctx.user_quotes[user_account_index]
                }
            },
            user_owner: &dex_test_ctx.user_owners[user_account_index].pubkey(),
            discount_token_account: None,
        },
        new_order::Params {
            side: side as u8,
            limit_price,
            max_base_qty,
            max_quote_qty,
            order_type: new_order::OrderType::Limit as u8,
            self_trade_behavior: agnostic_orderbook::state::SelfTradeBehavior::DecrementTake as u8,
            match_limit: 10,
            _padding: [0; 5],
        },
    );
    sign_send_instructions(
        pgr_test_ctx,
        vec![new_order_instruction],
        vec![&dex_test_ctx.user_owners[user_account_index]],
    )
    .await
    .unwrap();
}

#[allow(clippy::too_many_arguments)]
pub async fn serum_dex_new_order(
    pgr_test_ctx: &mut ProgramTestContext,
    aob_dex_test_ctx: &AobDexTestContext,
    serum_test_ctx: &SerumTestContext,
    serum_dex_program_id: Pubkey,
    side: serum_dex::matching::Side,
    limit_price: u64,
    max_coin_qty: u64,
    max_native_pc_qty_including_fees: u64,
    open_orders_index: usize,
) {
    // New order on old Serum Dex
    let new_order_instruction = serum_dex::instruction::new_order(
        &serum_test_ctx.serum_market.market_key.pubkey(),
        &serum_test_ctx.open_orders[open_orders_index],
        &serum_test_ctx.serum_market.req_q_key.pubkey(),
        &serum_test_ctx.serum_market.event_q_key.pubkey(),
        &serum_test_ctx.serum_market.bids_key.pubkey(),
        &serum_test_ctx.serum_market.asks_key.pubkey(),
        &match side {
            serum_dex::matching::Side::Bid => aob_dex_test_ctx.user_quotes[open_orders_index],
            serum_dex::matching::Side::Ask => aob_dex_test_ctx.user_bases[open_orders_index],
        },
        &aob_dex_test_ctx.user_owners[open_orders_index].pubkey(),
        &serum_test_ctx.serum_market.coin_vault,
        &serum_test_ctx.serum_market.pc_vault,
        &spl_token::ID,
        &sysvar::rent::ID,
        None,
        &serum_dex_program_id,
        side,
        limit_price.try_into().unwrap(),
        max_coin_qty.try_into().unwrap(),
        serum_dex::matching::OrderType::Limit,
        0,
        serum_dex::instruction::SelfTradeBehavior::DecrementTake,
        10,
        max_native_pc_qty_including_fees.try_into().unwrap(),
    )
    .unwrap();
    sign_send_instructions(
        pgr_test_ctx,
        vec![new_order_instruction],
        vec![&aob_dex_test_ctx.user_owners[open_orders_index]],
    )
    .await
    .unwrap();
}
