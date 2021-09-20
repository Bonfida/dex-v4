use std::{cell::RefCell, rc::Rc};

use agnostic_orderbook::{
    msrm_token,
    state::{Event, EventQueue, EventQueueHeader, MarketState},
};
use borsh::BorshDeserialize;
use dex_v3::{
    instruction::consume_events,
    state::{CallBackInfo, DexState},
    CALLBACK_INFO_LEN,
};
use error::CrankError;
use solana_client::{
    client_error::ClientError, rpc_client::RpcClient, rpc_config::RpcSendTransactionConfig,
};
use solana_program::pubkey::Pubkey;
use solana_sdk::{
    commitment_config::{CommitmentConfig, CommitmentLevel},
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;

pub mod error;
pub mod utils;

pub struct Context {
    pub program_id: Pubkey,
    pub market: Pubkey,
    pub reward_target: Pubkey,
    pub fee_payer: Keypair,
    pub cranking_authority: Keypair,
    pub endpoint: String,
}

pub const MAX_ITERATIONS: u64 = 10;

impl Context {
    pub fn crank(self) {
        let connection =
            RpcClient::new_with_commitment(self.endpoint.clone(), CommitmentConfig::confirmed());

        let market_state_data = connection
            .get_account_data(&self.market)
            .map_err(|_| CrankError::ConnectionError)
            .unwrap();
        let market_state = DexState::deserialize(&mut (&market_state_data as &[u8])).unwrap();

        let market_signer = Pubkey::create_program_address(
            &[&self.market.to_bytes(), &[market_state.signer_nonce]],
            &self.program_id,
        )
        .unwrap();
        let orderbook_data = connection
            .get_account_data(&market_state.orderbook)
            .unwrap();
        let orderbook =
            agnostic_orderbook::state::MarketState::deserialize(&mut (&orderbook_data as &[u8]))
                .unwrap();
        let msrm_token_account =
            get_associated_token_address(&self.cranking_authority.pubkey(), &msrm_token::ID);
        loop {
            let res = self.consume_events_iteration(
                &connection,
                &orderbook,
                &market_state,
                &market_signer,
                &msrm_token_account,
            );
            println!("{:#?}", res);
        }
    }

    pub fn consume_events_iteration(
        &self,
        connection: &RpcClient,
        orderbook: &MarketState,
        market_state: &DexState,
        market_signer: &Pubkey,
        msrm_token_account: &Pubkey,
    ) -> Result<Signature, ClientError> {
        let mut event_queue_data = connection.get_account_data(&orderbook.event_queue)?;
        let event_queue_header =
            EventQueueHeader::deserialize(&mut (&event_queue_data as &[u8])).unwrap();
        let length = event_queue_header.count as usize;
        let event_queue = EventQueue::new(
            event_queue_header,
            Rc::new(RefCell::new(&mut event_queue_data)),
            CALLBACK_INFO_LEN as usize,
        );
        let mut user_accounts = Vec::with_capacity(length << 1);
        for e in event_queue.iter() {
            match e {
                Event::Fill {
                    taker_side: _,
                    maker_order_id: _,
                    quote_size: _,
                    base_size: _,
                    maker_callback_info,
                    taker_callback_info,
                } => {
                    let maker_callback_info =
                        CallBackInfo::deserialize(&mut (&maker_callback_info as &[u8])).unwrap();
                    user_accounts.push(maker_callback_info.user_account);
                }
                Event::Out {
                    side: _,
                    order_id: _,
                    base_size: _,
                    delete: _,
                    callback_info,
                } => {
                    let callback_info =
                        CallBackInfo::deserialize(&mut (&callback_info as &[u8])).unwrap();
                    user_accounts.push(callback_info.user_account);
                }
            }
        }

        let consume_events_instruction = consume_events(
            self.program_id,
            market_state.aaob_program,
            self.market,
            *market_signer,
            market_state.orderbook,
            orderbook.event_queue,
            self.reward_target,
            *msrm_token_account,
            self.cranking_authority.pubkey(),
            &user_accounts,
            consume_events::Params {
                max_iterations: MAX_ITERATIONS,
            },
        );

        let mut transaction = Transaction::new_with_payer(
            &[consume_events_instruction],
            Some(&self.fee_payer.pubkey()),
        );
        let (recent_blockhash, _) = connection.get_recent_blockhash()?;
        transaction.partial_sign(
            &[&self.fee_payer, &self.cranking_authority],
            recent_blockhash,
        );
        connection.send_transaction_with_config(
            &transaction,
            RpcSendTransactionConfig {
                skip_preflight: false,
                preflight_commitment: Some(CommitmentLevel::Processed),
                ..RpcSendTransactionConfig::default()
            },
        )
    }
}
