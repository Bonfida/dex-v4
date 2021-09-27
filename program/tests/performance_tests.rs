pub mod common;

use crate::common::performance_test_utils::aob_dex_new_order;
use crate::common::performance_test_utils::create_aob_dex;
use crate::common::performance_test_utils::initialize_serum_market_accounts;
use crate::common::performance_test_utils::serum_dex_new_order;
use solana_program::pubkey::Pubkey;
use solana_program_test::{processor, ProgramTest};

const NB_INSTRUCTIONS: usize = 1_000;

#[test]
fn test_dex_perf() {
    // Create program and test environment
    let dex_program_id = Pubkey::new_unique();
    let aaob_program_id = Pubkey::new_unique();
    let serum_dex_program_id = Pubkey::new_unique(); // The old serum version

    println!("Serum_dex_key {:?}", serum_dex_program_id);
    println!("Aob_dex_key {:?}", dex_program_id);

    let mut program_test = ProgramTest::new(
        "dex_v3",
        dex_program_id,
        processor!(dex_v3::entrypoint::process_instruction),
    );
    program_test.add_program(
        "agnostic_orderbook",
        aaob_program_id,
        processor!(agnostic_orderbook::entrypoint::process_instruction),
    );
    program_test.add_program("serum_dex", serum_dex_program_id, None);

    let rt = tokio::runtime::Runtime::new().unwrap();

    let (aob_dex_test_ctx, mut pgr_test_ctx) = rt.block_on(create_aob_dex(
        program_test,
        aaob_program_id,
        dex_program_id,
    ));

    let serum_dex_test_ctx = rt
        .block_on(initialize_serum_market_accounts(
            &mut pgr_test_ctx,
            &aob_dex_test_ctx,
            serum_dex_program_id,
        ))
        .unwrap();

    // Order params:
    let side = agnostic_orderbook::state::Side::Ask;
    let limit_price = 1000;
    let max_base_qty = 1000;
    let max_quote_qty = 1000;

    for i in 0..NB_INSTRUCTIONS {
        eprintln!("Progress {:?} %", (100 * i / NB_INSTRUCTIONS) as u64);
        rt.block_on(aob_dex_new_order(
            &mut pgr_test_ctx,
            &aob_dex_test_ctx,
            side,
            limit_price,
            max_base_qty,
            max_quote_qty,
        ));

        rt.block_on(serum_dex_new_order(
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
        ));

        pgr_test_ctx.last_blockhash = rt
            .block_on(pgr_test_ctx.banks_client.get_recent_blockhash())
            .unwrap();
    }
}
