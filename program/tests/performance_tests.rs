#![cfg(not(feature = "test-bpf"))]

pub mod common;

use crate::common::performance_test_utils::aob_dex_new_order;
use crate::common::performance_test_utils::create_aob_dex;
use crate::common::performance_test_utils::initialize_serum_market_accounts;
use crate::common::performance_test_utils::serum_dex_new_order;
use crate::common::performance_test_utils::NB_USER_ACCS;
use rand::Rng;
use rand_distr::Uniform;
use solana_program::pubkey::Pubkey;

const NB_INSTRUCTIONS: u32 = 1_000;

#[tokio::test]
async fn test_dex_perf() {
    use solana_program_test::{processor, ProgramTest};

    // Create program and test environment
    let aaob_program_id = Pubkey::new_unique();
    let dex_program_id = Pubkey::new_unique();
    let serum_dex_program_id = Pubkey::new_unique(); // The old serum version

    println!("Serum_dex_key {:?}", serum_dex_program_id);
    println!("Aob_dex_key {:?}", dex_program_id);

    let mut program_test = ProgramTest::new(
        "dex_v4",
        dex_program_id,
        processor!(dex_v4::entrypoint::process_instruction),
    );
    program_test.add_program(
        "agnostic_orderbook",
        agnostic_orderbook::ID,
        processor!(agnostic_orderbook::entrypoint::process_instruction),
    );
    program_test.add_program(
        "serum_dex",
        serum_dex_program_id,
        None,
        // processor!(
        //     |program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]| {
        //         Ok(serum_dex::state::State::process(
        //             program_id, accounts, input,
        //         )?)
        //     }
        // ),
    );

    let (aob_dex_test_ctx, mut pgr_test_ctx) =
        create_aob_dex(program_test, aaob_program_id, dex_program_id).await;

    let serum_dex_test_ctx = initialize_serum_market_accounts(
        &mut pgr_test_ctx,
        &aob_dex_test_ctx,
        serum_dex_program_id,
    )
    .await
    .unwrap();

    // Order params:
    let rng = &mut rand::thread_rng();
    let uniform = Uniform::new(800, 1200);
    let mut side;
    let max_base_qty = 1000;
    let max_quote_qty = 1000;

    for i in 0..NB_INSTRUCTIONS {
        eprintln!("Progress {:?} %", (100 * i / NB_INSTRUCTIONS) as u64);

        let limit_price = rng.sample(uniform);

        match i % 2 {
            0 => side = agnostic_orderbook::state::Side::Bid,
            1 => side = agnostic_orderbook::state::Side::Ask,
            _ => unreachable!(),
        }

        aob_dex_new_order(
            &mut pgr_test_ctx,
            &aob_dex_test_ctx,
            side,
            limit_price,
            max_base_qty,
            max_quote_qty,
            (i % NB_USER_ACCS) as usize,
            dex_program_id,
        )
        .await;

        serum_dex_new_order(
            &mut pgr_test_ctx,
            &aob_dex_test_ctx,
            &serum_dex_test_ctx,
            serum_dex_program_id,
            match side {
                agnostic_orderbook::state::Side::Bid => serum_dex::matching::Side::Bid,
                agnostic_orderbook::state::Side::Ask => serum_dex::matching::Side::Ask,
            },
            limit_price,
            max_base_qty,
            max_quote_qty,
            (i % NB_USER_ACCS) as usize,
        )
        .await;

        pgr_test_ctx.last_blockhash = pgr_test_ctx
            .banks_client
            .get_recent_blockhash()
            .await
            .unwrap();
    }
}
