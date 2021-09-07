use agnostic_orderbook::instruction::create_market;
use solana_program::instruction::Instruction;
use solana_program::pubkey::Pubkey;
use solana_program::system_instruction::create_account;
use solana_program_test::ProgramTestContext;
use solana_sdk::signature::Signer;
use solana_sdk::{signature::Keypair, transaction::Transaction, transport::TransportError};
use spl_associated_token_account::{create_associated_token_account, get_associated_token_address};

pub async fn sign_send_instructions(
    ctx: &mut ProgramTestContext,
    instructions: Vec<Instruction>,
    signers: Vec<&Keypair>,
) -> Result<(), TransportError> {
    let mut transaction = Transaction::new_with_payer(&instructions, Some(&ctx.payer.pubkey()));
    let mut payer_signers = vec![&ctx.payer];
    for s in signers {
        payer_signers.push(s);
    }
    transaction.partial_sign(&payer_signers, ctx.last_blockhash);
    ctx.banks_client.process_transaction(transaction).await
}

pub async fn create_associated_token(
    mut prg_test_ctx: &mut ProgramTestContext,
    mint: &Pubkey,
    owner: &Pubkey,
) -> Pubkey {
    let create_associated_instruction =
        create_associated_token_account(&prg_test_ctx.payer.pubkey(), mint, owner);
    let associated_key = get_associated_token_address(owner, mint);

    sign_send_instructions(
        &mut prg_test_ctx,
        vec![create_associated_instruction],
        vec![],
    )
    .await
    .unwrap();
    associated_key
}

/// Creates the accounts needed for the AAOB market testing and returns the
/// address of the market.
pub async fn create_market_and_accounts(
    mut prg_test_ctx: &mut ProgramTestContext,
    agnostic_orderbook_program_id: Pubkey,
    caller_authority: &Keypair,
) -> Pubkey {
    // Create market state account
    let market_account = Keypair::new();
    let create_market_account_instruction = create_account(
        &prg_test_ctx.payer.pubkey(),
        &market_account.pubkey(),
        1_000_000,
        1_000_000,
        &agnostic_orderbook_program_id,
    );
    sign_send_instructions(
        &mut prg_test_ctx,
        vec![create_market_account_instruction],
        vec![&market_account],
    )
    .await
    .unwrap();

    // Create event queue account
    let event_queue_account = Keypair::new();
    let create_event_queue_account_instruction = create_account(
        &prg_test_ctx.payer.pubkey(),
        &event_queue_account.pubkey(),
        1_000_000,
        1_000_000,
        &agnostic_orderbook_program_id,
    );
    sign_send_instructions(
        &mut prg_test_ctx,
        vec![create_event_queue_account_instruction],
        vec![&event_queue_account],
    )
    .await
    .unwrap();

    // Create bids account
    let bids_account = Keypair::new();
    let create_bids_account_instruction = create_account(
        &prg_test_ctx.payer.pubkey(),
        &bids_account.pubkey(),
        1_000_000,
        1_000_000,
        &agnostic_orderbook_program_id,
    );
    sign_send_instructions(
        &mut prg_test_ctx,
        vec![create_bids_account_instruction],
        vec![&bids_account],
    )
    .await
    .unwrap();

    // Create asks account
    let asks_account = Keypair::new();
    let create_asks_account_instruction = create_account(
        &prg_test_ctx.payer.pubkey(),
        &asks_account.pubkey(),
        1_000_000,
        1_000_000,
        &agnostic_orderbook_program_id,
    );
    sign_send_instructions(
        &mut prg_test_ctx,
        vec![create_asks_account_instruction],
        vec![&asks_account],
    )
    .await
    .unwrap();

    // Create Market
    let create_market_instruction = create_market(
        agnostic_orderbook_program_id,
        market_account.pubkey(),
        event_queue_account.pubkey(),
        bids_account.pubkey(),
        asks_account.pubkey(),
        create_market::Params {
            caller_authority: caller_authority.pubkey(),
            callback_info_len: 32,
        },
    );
    sign_send_instructions(&mut prg_test_ctx, vec![create_market_instruction], vec![])
        .await
        .unwrap();

    market_account.pubkey()
}
