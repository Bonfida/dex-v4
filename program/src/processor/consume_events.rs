use std::{convert::TryInto, rc::Rc};

use agnostic_orderbook::{
    instruction::consume_events,
    state::{Event, EventQueue, EventQueueHeader, Side},
};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{
    error::DexError,
    state::{CallBackInfo, DexState, UserAccount},
    utils::check_account_key,
};

use super::CALLBACK_INFO_LEN;

#[derive(BorshDeserialize, BorshSerialize)]
/**
The required arguments for a create_market instruction.
*/
pub struct Params {
    pub max_iterations: u64,
}

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub enum OrderType {
    Limit,
    ImmediateOrCancel,
    FillOrKill,
    PostOnly,
}

struct Accounts<'a, 'b: 'a> {
    aaob_program: &'a AccountInfo<'b>,
    market: &'a AccountInfo<'b>,
    market_signer: &'a AccountInfo<'b>,
    orderbook: &'a AccountInfo<'b>,
    event_queue: &'a AccountInfo<'b>,
    reward_target: &'a AccountInfo<'b>,
    msrm_token_account: &'a AccountInfo<'b>,
    msrm_token_account_owner: &'a AccountInfo<'b>,
    user_accounts: &'a [AccountInfo<'b>],
}

impl<'a, 'b: 'a> Accounts<'a, 'b> {
    pub fn parse(
        _program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        let a = Self {
            aaob_program: next_account_info(accounts_iter)?,
            market: next_account_info(accounts_iter)?,
            market_signer: next_account_info(accounts_iter)?,
            orderbook: next_account_info(accounts_iter)?,
            event_queue: next_account_info(accounts_iter)?,
            reward_target: next_account_info(accounts_iter)?,
            msrm_token_account: next_account_info(accounts_iter)?,
            msrm_token_account_owner: next_account_info(accounts_iter)?,
            user_accounts: accounts_iter.as_slice(),
        };

        Ok(a)
    }
}

pub(crate) fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    params: Params,
) -> ProgramResult {
    let accounts = Accounts::parse(program_id, accounts)?;

    let Params { max_iterations } = params;

    let mut market_state =
        DexState::deserialize(&mut (&accounts.market.data.borrow() as &[u8]))?.check()?;

    let mut market_data: &mut [u8] = &mut accounts.market.data.borrow_mut();
    market_state.serialize(&mut market_data).unwrap();

    let event_queue_header =
        EventQueueHeader::deserialize(&mut (&accounts.event_queue.data.borrow() as &[u8]))?;
    let event_queue = EventQueue::new(
        event_queue_header,
        Rc::clone(&accounts.event_queue.data),
        CALLBACK_INFO_LEN as usize,
    );

    check_accounts(program_id, &market_state, &accounts).unwrap();

    let mut total_iterations = 0;

    for event in event_queue.iter().take(max_iterations as usize) {
        if consume_event(accounts.user_accounts, event, &mut market_state).is_err() {
            break;
        }
        total_iterations += 1;
    }

    if total_iterations == 0 {
        msg!("Failed to complete one iteration");
        return Err(DexError::NoOp.into());
    }

    let pop_events_instruction = consume_events(
        *accounts.aaob_program.key,
        *accounts.orderbook.key,
        *accounts.market_signer.key,
        *accounts.event_queue.key,
        *accounts.reward_target.key,
        *accounts.msrm_token_account.key,
        *accounts.msrm_token_account_owner.key,
        agnostic_orderbook::instruction::consume_events::Params {
            number_of_entries_to_consume: total_iterations,
        },
    );

    invoke_signed(
        &pop_events_instruction,
        &[
            accounts.aaob_program.clone(),
            accounts.orderbook.clone(),
            accounts.event_queue.clone(),
            accounts.market_signer.clone(),
            accounts.reward_target.clone(),
        ],
        &[&[
            &accounts.market.key.to_bytes(),
            &[market_state.signer_nonce],
        ]],
    )?;
    Ok(())
}

fn check_accounts(
    program_id: &Pubkey,
    market_state: &DexState,
    accounts: &Accounts,
) -> ProgramResult {
    let market_signer = Pubkey::create_program_address(
        &[
            &accounts.market.key.to_bytes(),
            &[market_state.signer_nonce],
        ],
        program_id,
    )?;
    check_account_key(accounts.market_signer, &market_signer).unwrap();
    check_account_key(accounts.orderbook, &market_state.orderbook).unwrap();
    check_account_key(accounts.aaob_program, &market_state.aaob_program).unwrap();
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
            asset_size,
            maker_callback_info,
            taker_callback_info,
        } => {
            msg!("T {:?}", taker_callback_info);
            msg!("M {:?}", maker_callback_info);
            let taker_info =
                CallBackInfo::deserialize(&mut (&taker_callback_info as &[u8])).unwrap();
            let maker_info =
                CallBackInfo::deserialize(&mut (&maker_callback_info as &[u8])).unwrap();
            let taker_account_info = &accounts[accounts
                .binary_search_by_key(&taker_info.user_account, |k| *k.key)
                .map_err(|_| DexError::MissingUserAccount)?];
            if taker_info.user_account == maker_info.user_account {
                // Self trade scenario
                // TODO: bug when fee tier changes for a particular user account
                let mut taker_account = UserAccount::parse(taker_account_info).unwrap();
                taker_account.header.base_token_free = taker_account
                    .header
                    .base_token_free
                    .checked_add(asset_size)
                    .unwrap();
                taker_account.header.quote_token_locked = taker_account
                    .header
                    .quote_token_locked
                    .checked_sub(quote_size)
                    .unwrap();
                taker_account.header.quote_token_free = taker_account
                    .header
                    .quote_token_free
                    .checked_add(quote_size)
                    .unwrap();
                taker_account.header.base_token_locked = taker_account
                    .header
                    .quote_token_free
                    .checked_sub(asset_size)
                    .unwrap();
            } else {
                let maker_account_info = &accounts[accounts
                    .binary_search_by_key(&maker_info.user_account, |k| *k.key)
                    .map_err(|_| DexError::MissingUserAccount)?];
                let mut taker_account = UserAccount::parse(taker_account_info).unwrap();
                let mut maker_account = UserAccount::parse(maker_account_info).unwrap();
                match taker_side {
                    Side::Bid => {
                        let maker_rebate = maker_info.fee_tier.maker_rebate(quote_size);
                        taker_account.header.base_token_free = taker_account
                            .header
                            .base_token_free
                            .checked_add(asset_size)
                            .unwrap();
                        taker_account.header.quote_token_locked = taker_account
                            .header
                            .quote_token_locked
                            .checked_sub(quote_size)
                            .unwrap();
                        maker_account.header.quote_token_free = maker_account
                            .header
                            .quote_token_free
                            .checked_add(quote_size + maker_rebate)
                            .unwrap();
                        maker_account.header.accumulated_rebates += maker_rebate;
                        maker_account.header.base_token_locked = maker_account
                            .header
                            .quote_token_free
                            .checked_sub(asset_size)
                            .unwrap();
                        market_state.accumulated_fees += taker_info
                            .fee_tier
                            .taker_fee(quote_size)
                            .checked_sub(maker_rebate)
                            .unwrap();
                    }
                    Side::Ask => {
                        let taker_fee = taker_info.fee_tier.taker_fee(quote_size);
                        let quote_size_without_fees = quote_size - taker_fee;
                        let maker_rebate = maker_info.fee_tier.maker_rebate(quote_size);
                        taker_account.header.quote_token_free = taker_account
                            .header
                            .quote_token_free
                            .checked_add(quote_size_without_fees)
                            .unwrap();
                        taker_account.header.base_token_locked = taker_account
                            .header
                            .base_token_locked
                            .checked_sub(asset_size)
                            .unwrap();
                        maker_account.header.base_token_free = maker_account
                            .header
                            .base_token_free
                            .checked_add(asset_size)
                            .unwrap();
                        maker_account.header.quote_token_locked = maker_account
                            .header
                            .quote_token_locked
                            .checked_sub(quote_size - maker_rebate)
                            .unwrap();
                        maker_account.header.accumulated_rebates += maker_rebate;
                        market_state.accumulated_fees +=
                            taker_fee.checked_sub(maker_rebate).unwrap();
                    }
                };
                maker_account.write();
                taker_account.write();
                market_state.quote_volume += quote_size;
                market_state.base_volume += asset_size;
            }
        }
        Event::Out {
            side: _,
            order_id,
            asset_size: _,
            callback_info,
            delete,
        } => {
            if delete {
                let user_key = Pubkey::new_from_array(callback_info.try_into().unwrap());
                let user_account_info = &accounts[accounts
                    .binary_search_by_key(&user_key, |k| *k.key)
                    .map_err(|_| DexError::MissingUserAccount)?];
                let mut user_account = UserAccount::parse(user_account_info).unwrap();
                let order_index = user_account.find_order_index(order_id).unwrap();
                user_account.remove_order(order_index).unwrap();
                user_account.write();
            }
        }
    };
    Ok(())
}
