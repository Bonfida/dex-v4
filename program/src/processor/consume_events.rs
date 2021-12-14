use std::rc::Rc;

use crate::{
    error::DexError,
    state::{CallBackInfo, DexState, UserAccount},
    utils::{check_account_key, check_account_owner, fp32_mul},
};
use agnostic_orderbook::{
    error::AoError,
    state::{Event, EventQueue, EventQueueHeader, Side},
};
use bonfida_utils::BorshSize;
use bonfida_utils::InstructionsAccount;
use borsh::BorshDeserialize;
use borsh::BorshSerialize;
use bytemuck::{try_from_bytes, Pod, Zeroable};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::{PrintProgramError, ProgramError},
    pubkey::Pubkey,
};

use super::CALLBACK_INFO_LEN;

#[derive(Copy, Clone, Zeroable, Pod, BorshDeserialize, BorshSerialize, BorshSize)]
#[repr(C)]
/**
The required arguments for a consume_events instruction.
*/
pub struct Params {
    /// The maximum number of events to consume
    pub max_iterations: u64,
}

#[derive(InstructionsAccount)]
pub struct Accounts<'a, T> {
    #[cons(writable)]
    pub market: &'a T,
    #[cons(writable)]
    pub orderbook: &'a T,
    #[cons(writable)]
    pub event_queue: &'a T,
    #[cons(writable)]
    pub reward_target: &'a T,
    #[cons(writable)]
    pub user_accounts: &'a [T],
}

impl<'a, 'b: 'a> Accounts<'a, AccountInfo<'b>> {
    pub fn parse(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        let a = Self {
            market: next_account_info(accounts_iter)?,
            orderbook: next_account_info(accounts_iter)?,
            event_queue: next_account_info(accounts_iter)?,
            reward_target: next_account_info(accounts_iter)?,
            user_accounts: accounts_iter.as_slice(),
        };

        check_account_owner(a.market, program_id, DexError::InvalidStateAccountOwner)?;

        Ok(a)
    }
}

pub(crate) fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let accounts = Accounts::parse(program_id, accounts)?;

    let Params { max_iterations } =
        try_from_bytes(instruction_data).map_err(|_| ProgramError::InvalidInstructionData)?;

    let mut market_state = DexState::get(accounts.market)?;

    let event_queue_header =
        EventQueueHeader::deserialize(&mut (&accounts.event_queue.data.borrow() as &[u8]))?;
    let event_queue = EventQueue::new(
        event_queue_header,
        Rc::clone(&accounts.event_queue.data),
        CALLBACK_INFO_LEN as usize,
    );

    check_accounts(&market_state, &accounts).unwrap();

    let mut total_iterations = 0;

    for event in event_queue.iter().take(*max_iterations as usize) {
        if consume_event(accounts.user_accounts, event, &mut market_state).is_err() {
            break;
        }
        total_iterations += 1;
    }

    if total_iterations == 0 {
        msg!("Failed to complete one iteration");
        return Err(DexError::NoOp.into());
    }

    let invoke_params = agnostic_orderbook::instruction::consume_events::Params {
        number_of_entries_to_consume: total_iterations,
    };
    let invoke_accounts = agnostic_orderbook::instruction::consume_events::Accounts {
        market: accounts.orderbook,
        event_queue: accounts.event_queue,
        authority: accounts.market, // No impact with AOB as a lib
        reward_target: accounts.reward_target,
    };

    if let Err(error) = agnostic_orderbook::instruction::consume_events::process(
        program_id,
        invoke_accounts,
        invoke_params,
    ) {
        error.print::<AoError>();
        return Err(DexError::AOBError.into());
    }

    Ok(())
}

fn check_accounts(market_state: &DexState, accounts: &Accounts<AccountInfo>) -> ProgramResult {
    check_account_key(
        accounts.orderbook,
        &market_state.orderbook,
        DexError::InvalidOrderbookAccount,
    )?;
    Ok(())
}

fn consume_event(
    accounts: &[AccountInfo],
    event: Event,
    market_state: &mut DexState,
) -> Result<(), DexError> {
    match event {
        Event::Fill {
            taker_side,
            maker_order_id: _,
            quote_size,
            base_size,
            maker_callback_info,
            taker_callback_info,
        } => {
            let taker_info =
                CallBackInfo::deserialize(&mut (&taker_callback_info as &[u8])).unwrap();
            let maker_info =
                CallBackInfo::deserialize(&mut (&maker_callback_info as &[u8])).unwrap();
            let maker_account_info = &accounts[accounts
                .binary_search_by_key(&maker_info.user_account, |k| *k.key)
                .map_err(|_| DexError::MissingUserAccount)?];
            let mut taker_account = UserAccount::get(maker_account_info).unwrap();
            if taker_info.user_account == maker_info.user_account {
                let maker_rebate = taker_info.fee_tier.maker_rebate(quote_size);
                taker_account.header.quote_token_free = taker_account
                    .header
                    .quote_token_free
                    .checked_add(maker_rebate)
                    .unwrap();

                match taker_side {
                    Side::Bid => {
                        taker_account.header.base_token_locked = taker_account
                            .header
                            .base_token_locked
                            .checked_sub(base_size)
                            .unwrap();
                    }
                    Side::Ask => {
                        taker_account.header.quote_token_locked = taker_account
                            .header
                            .quote_token_locked
                            .checked_sub(quote_size)
                            .unwrap();
                    }
                };

                // Update user accounts metrics
                taker_account.header.accumulated_maker_quote_volume = taker_account
                    .header
                    .accumulated_maker_quote_volume
                    .checked_add(quote_size)
                    .unwrap();
                taker_account.header.accumulated_maker_base_volume = taker_account
                    .header
                    .accumulated_maker_base_volume
                    .checked_add(base_size)
                    .unwrap();
                taker_account.header.accumulated_taker_quote_volume = taker_account
                    .header
                    .accumulated_taker_quote_volume
                    .checked_add(quote_size)
                    .unwrap();
                taker_account.header.accumulated_taker_base_volume = taker_account
                    .header
                    .accumulated_taker_base_volume
                    .checked_add(base_size)
                    .unwrap();
            } else {
                let mut maker_account = UserAccount::get(maker_account_info).unwrap();
                match taker_side {
                    Side::Bid => {
                        let maker_rebate = maker_info.fee_tier.maker_rebate(quote_size);
                        maker_account.header.quote_token_free = maker_account
                            .header
                            .quote_token_free
                            .checked_add(quote_size + maker_rebate)
                            .unwrap();
                        maker_account.header.accumulated_rebates += maker_rebate;
                        maker_account.header.base_token_locked = maker_account
                            .header
                            .base_token_locked
                            .checked_sub(base_size)
                            .unwrap();
                        market_state.accumulated_fees += taker_info
                            .fee_tier
                            .taker_fee(quote_size)
                            .checked_sub(maker_rebate)
                            .unwrap();
                    }
                    Side::Ask => {
                        let taker_fee = taker_info.fee_tier.taker_fee(quote_size);
                        let maker_rebate = maker_info.fee_tier.maker_rebate(quote_size);
                        maker_account.header.base_token_free = maker_account
                            .header
                            .base_token_free
                            .checked_add(base_size)
                            .unwrap();
                        maker_account.header.quote_token_locked = maker_account
                            .header
                            .quote_token_locked
                            .checked_sub(quote_size)
                            .unwrap();
                        maker_account
                            .header
                            .quote_token_free
                            .checked_add(maker_rebate)
                            .unwrap();
                        maker_account.header.accumulated_rebates += maker_rebate;
                        market_state.accumulated_fees +=
                            taker_fee.checked_sub(maker_rebate).unwrap();
                    }
                };

                // Update user accounts metrics
                maker_account.header.accumulated_maker_quote_volume = maker_account
                    .header
                    .accumulated_maker_quote_volume
                    .checked_add(quote_size)
                    .unwrap();
                maker_account.header.accumulated_maker_base_volume = maker_account
                    .header
                    .accumulated_maker_base_volume
                    .checked_add(base_size)
                    .unwrap();

                market_state.quote_volume =
                    market_state.quote_volume.checked_add(quote_size).unwrap();
                market_state.base_volume = market_state.base_volume.checked_add(base_size).unwrap();
            }
        }
        Event::Out {
            side,
            order_id,
            base_size,
            callback_info,
            delete,
        } => {
            if !delete && base_size == 0 {
                return Ok(());
            }

            let user_callback_info =
                CallBackInfo::deserialize(&mut (&callback_info as &[u8])).unwrap();
            let user_account_info = &accounts[accounts
                .binary_search_by_key(&user_callback_info.user_account, |k| *k.key)
                .map_err(|_| DexError::MissingUserAccount)?];
            let mut user_account = UserAccount::get(user_account_info).unwrap();

            if base_size != 0 {
                match side {
                    Side::Ask => {
                        user_account.header.base_token_free = user_account
                            .header
                            .base_token_free
                            .checked_add(base_size)
                            .unwrap()
                    }
                    Side::Bid => {
                        let price = (order_id >> 64) as u64;
                        let qty_to_transfer = fp32_mul(base_size, price);
                        user_account.header.quote_token_free = user_account
                            .header
                            .quote_token_free
                            .checked_add(qty_to_transfer.unwrap())
                            .unwrap();
                    }
                }
            }
            if delete {
                let order_index = user_account.find_order_index(order_id).unwrap();
                user_account.remove_order(order_index).unwrap();
            }
        }
    };
    Ok(())
}
