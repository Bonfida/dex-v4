use std::{cell::RefCell, rc::Rc};

use agnostic_orderbook::state::{
    Event, EventQueue, EventQueueHeader, MarketState, MARKET_STATE_LEN,
};
use borsh::BorshDeserialize;
use dex_v4::instruction::consume_events;
use dex_v4::{
    instruction::consume_events::Accounts,
    state::{CallBackInfo, DexState, DEX_STATE_LEN},
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

pub mod error;
pub mod utils;

pub struct Context {
    pub program_id: Pubkey,
    pub market: Pubkey,
    pub reward_target: Pubkey,
    pub fee_payer: Keypair,
    pub endpoint: String,
}

pub const MAX_ITERATIONS: u64 = 10;
pub const MAX_NUMBER_OF_USER_ACCOUNTS: usize = 20;

impl Context {
    pub fn crank(self) {
        let connection =
            RpcClient::new_with_commitment(self.endpoint.clone(), CommitmentConfig::confirmed());

        let market_state_data = connection
            .get_account_data(&self.market)
            .map_err(|_| CrankError::ConnectionError)
            .unwrap();
        let market_state =
            bytemuck::try_from_bytes::<DexState>(&market_state_data[..DEX_STATE_LEN]).unwrap();

        let orderbook_data = connection
            .get_account_data(&Pubkey::new(&market_state.orderbook))
            .unwrap();
        let orderbook =
            bytemuck::try_from_bytes::<MarketState>(&orderbook_data[..MARKET_STATE_LEN]).unwrap();
        loop {
            let res = self.consume_events_iteration(&connection, &orderbook, &market_state);
            println!("{:#?}", res);
        }
    }

    pub fn consume_events_iteration(
        &self,
        connection: &RpcClient,
        orderbook: &MarketState,
        market_state: &DexState,
    ) -> Result<Signature, ClientError> {
        let mut event_queue_data =
            connection.get_account_data(&Pubkey::new(&orderbook.event_queue))?;
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
                    taker_callback_info: _,
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

        user_accounts.truncate(MAX_NUMBER_OF_USER_ACCOUNTS);

        // We don't use the default sort since the initial ordering of the pubkeys is completely random
        user_accounts.sort_unstable();
        // Since the array is sorted, this removes all duplicate accounts, which shrinks the array.
        user_accounts.dedup();

        let consume_events_instruction = consume_events(
            self.program_id,
            Accounts {
                orderbook: &Pubkey::new(&market_state.orderbook),
                market: &self.market,
                event_queue: &Pubkey::new(&orderbook.event_queue),
                reward_target: &self.reward_target,
                user_accounts: &user_accounts,
            },
            consume_events::Params {
                max_iterations: MAX_ITERATIONS,
            },
        );

        let mut transaction = Transaction::new_with_payer(
            &[consume_events_instruction],
            Some(&self.fee_payer.pubkey()),
        );
        let (recent_blockhash, _) = connection.get_recent_blockhash()?;
        transaction.partial_sign(&[&self.fee_payer], recent_blockhash);
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
