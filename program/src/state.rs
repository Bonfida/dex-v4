use borsh::{BorshDeserialize, BorshSerialize};
use bytemuck::{try_cast_slice_mut, try_from_bytes_mut, Pod, Zeroable};
use num_derive::{FromPrimitive, ToPrimitive};
use solana_program::{
    account_info::AccountInfo, msg, program_error::ProgramError, program_pack::Pack, pubkey::Pubkey,
};
use std::cell::RefMut;

use crate::{
    error::DexError,
    processor::{MSRM_MINT, SRM_MINT},
    utils::{fp32_div, fp32_mul, FP_32_ONE},
};

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, PartialEq, Copy)]
#[allow(missing_docs)]
#[repr(u64)]
pub enum AccountTag {
    Uninitialized,
    DexState,
    UserAccount,
    Closed,
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Copy, PartialEq, FromPrimitive, ToPrimitive)]
#[repr(u8)]
#[allow(missing_docs)]
pub enum Side {
    Bid,
    Ask,
}

/// This enum describes different supported behaviors for handling self trading scenarios
#[derive(BorshDeserialize, BorshSerialize, PartialEq, Clone, Copy)]
#[repr(u64)]
pub enum SelfTradeBehavior {
    /// Decrement take means that both the maker and taker sides of the matched orders are decremented.
    ///
    /// This is equivalent to a normal order match, except for the fact that no fees are applies.
    DecrementTake,
    /// Cancels the maker side of the order.
    CancelProvide,
    /// Cancels the whole transaction as soon as a self-matching scenario is encountered.
    AbortTransaction,
}

/// The primary market state object
#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct DexState {
    /// This u64 is used to verify and version the dex state
    pub tag: u64,
    /// The mint key of the base token
    pub base_mint: [u8; 32],
    /// The mint key of the quote token
    pub quote_mint: [u8; 32],
    /// The SPL token account holding the market's base tokens
    pub base_vault: [u8; 32],
    /// The SPL token account holding the market's quote tokens
    pub quote_vault: [u8; 32],
    /// The asset agnostic orderbook address
    pub orderbook: [u8; 32],
    /// The market admin which can recuperate all transaction fees
    pub admin: [u8; 32],
    /// The market's creation timestamp on the Solana runtime clock.
    pub creation_timestamp: i64,
    /// The market's total historical volume in base token
    pub base_volume: u64,
    /// The market's total historical volume in quote token
    pub quote_volume: u64,
    /// The market's fees which are available for extraction by the market admin
    pub accumulated_fees: u64,
    /// The market's minimum allowed order size in base token amount
    pub min_base_order_size: u64,
    /// The signer nonce is necessary for the market to perform as a signing entity
    pub signer_nonce: u64,
}

/// Size in bytes of the dex state object
pub const DEX_STATE_LEN: usize = 248;

impl DexState {
    pub(crate) fn get<'a, 'b: 'a>(
        account_info: &'a AccountInfo<'b>,
    ) -> Result<RefMut<'a, Self>, ProgramError> {
        let a = Self::get_unchecked(account_info);
        if a.tag != AccountTag::DexState as u64 {
            return Err(ProgramError::InvalidAccountData);
        };
        Ok(a)
    }

    pub(crate) fn get_unchecked<'a, 'b: 'a>(account_info: &'a AccountInfo<'b>) -> RefMut<'a, Self> {
        let a = RefMut::map(account_info.data.borrow_mut(), |s| {
            try_from_bytes_mut::<Self>(&mut s[0..DEX_STATE_LEN]).unwrap()
        });
        a
    }
}

/// This header describes a user account's state
#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct UserAccountHeader {
    /// This byte is used to verify and version the dex state
    pub tag: u64,
    /// The user account's assocatied DEX market
    pub market: [u8; 32],
    /// The user account owner's wallet
    pub owner: [u8; 32],
    /// The amount of base token available for settlement
    pub base_token_free: u64,
    /// The amount of base token currently locked in the orderbook
    pub base_token_locked: u64,
    /// The amount of quote token available for settlement
    pub quote_token_free: u64,
    /// The amount of quote token currently locked in the orderbook
    pub quote_token_locked: u64,
    /// The all time quantity of rebates accumulated by this user account.
    ///
    /// The actual rebates will always be transfer to the user account's main balance. This field is just a metric.
    pub accumulated_rebates: u64,
    /// The accumulated maker quote volume of the user. This field is just a metric.
    pub accumulated_maker_quote_volume: u64,
    /// The accumulated maker quote volume of the user. This field is just a metric.
    pub accumulated_maker_base_volume: u64,
    /// The accumulated taker quote volume of the user. This field is just a metric.
    pub accumulated_taker_quote_volume: u64,
    /// The accumulated taker quote volume of the user. This field is just a metric.
    pub accumulated_taker_base_volume: u64,
    /// We are forced to add padding here to keep the subsequent field as a u32 which maintains Borsh compatibility while respecting alignment constraints
    _padding: u32,
    /// The user account's number of active orders.
    pub number_of_orders: u32,
}

pub(crate) struct UserAccount<'a> {
    pub header: RefMut<'a, UserAccountHeader>,
    orders: RefMut<'a, [u128]>,
}

/// Size in bytes of the user account header object
pub const USER_ACCOUNT_HEADER_LEN: usize = 120;

impl UserAccountHeader {
    pub(crate) fn new(market: &Pubkey, owner: &Pubkey) -> Self {
        Self {
            tag: AccountTag::UserAccount as u64,
            market: market.to_bytes(),
            owner: owner.to_bytes(),
            base_token_free: 0,
            base_token_locked: 0,
            quote_token_free: 0,
            quote_token_locked: 0,
            number_of_orders: 0,
            accumulated_rebates: 0,
            _padding: 0,
        }
    }
}

impl<'a> UserAccount<'a> {
    pub(crate) fn get<'b: 'a>(account_info: &'a AccountInfo<'b>) -> Result<Self, ProgramError> {
        let a = Self::get_unchecked(account_info);
        if a.header.tag != AccountTag::UserAccount as u64 {
            return Err(ProgramError::InvalidAccountData);
        };
        Ok(a)
    }

    pub(crate) fn get_unchecked<'b: 'a>(account_info: &'a AccountInfo<'b>) -> Self {
        let a = RefMut::map_split(account_info.data.borrow_mut(), |s| {
            let (hd, tl) = s.split_at_mut(USER_ACCOUNT_HEADER_LEN);
            (
                try_from_bytes_mut(hd).unwrap(),
                try_cast_slice_mut(tl).unwrap(),
            )
        });
        Self {
            header: a.0,
            orders: a.1,
        }
    }
}

impl<'a> UserAccount<'a> {
    pub fn read_order(&self, order_index: usize) -> Result<u128, DexError> {
        if order_index >= self.header.number_of_orders as usize {
            return Err(DexError::InvalidOrderIndex);
        }
        Ok(self.orders[order_index])
    }

    pub fn remove_order(&mut self, order_index: usize) -> Result<(), DexError> {
        if order_index >= self.header.number_of_orders as usize {
            return Err(DexError::InvalidOrderIndex);
        }
        if self.header.number_of_orders - order_index as u32 != 1 {
            self.orders[order_index] = self.orders[self.header.number_of_orders as usize - 1];
        }
        self.header.number_of_orders -= 1;
        Ok(())
    }

    pub fn add_order(&mut self, order: u128) -> Result<(), DexError> {
        let slot = self
            .orders
            .get_mut(self.header.number_of_orders as usize)
            .ok_or(DexError::UserAccountFull)?;
        *slot = order;
        self.header.number_of_orders += 1;
        Ok(())
    }

    pub fn find_order_index(&self, order_id: u128) -> Result<usize, DexError> {
        let res = self
            .orders
            .iter()
            .enumerate()
            .find(|(_, b)| b == &&order_id)
            .ok_or(DexError::OrderNotFound)?
            .0;
        Ok(res)
    }
}

pub(crate) trait Order {
    const LEN: usize;
}

impl Order for u128 {
    const LEN: usize = 16;
}

#[doc(hidden)]
#[derive(BorshDeserialize, BorshSerialize, Debug, Clone, Copy)]
pub enum FeeTier {
    Base,
    Srm2,
    Srm3,
    Srm4,
    Srm5,
    Srm6,
    MSrm,
}

#[doc(hidden)]
impl FeeTier {
    pub fn from_srm_and_msrm_balances(srm_held: u64, msrm_held: u64) -> FeeTier {
        let one_srm = 1_000_000;
        match () {
            () if msrm_held >= 1 => FeeTier::MSrm,
            () if srm_held >= one_srm * 1_000_000 => FeeTier::Srm6,
            () if srm_held >= one_srm * 100_000 => FeeTier::Srm5,
            () if srm_held >= one_srm * 10_000 => FeeTier::Srm4,
            () if srm_held >= one_srm * 1_000 => FeeTier::Srm3,
            () if srm_held >= one_srm * 100 => FeeTier::Srm2,
            () => FeeTier::Base,
        }
    }

    pub fn get(account: &AccountInfo, expected_owner: &Pubkey) -> Result<Self, ProgramError> {
        let parsed_token_account = spl_token::state::Account::unpack(&account.data.borrow())?;
        if &parsed_token_account.owner != expected_owner {
            msg!("The discount token account must share its owner with the user account.");
            return Err(ProgramError::InvalidArgument);
        }
        let (srm_held, msrm_held) = match parsed_token_account.mint {
            a if a == MSRM_MINT => (0, parsed_token_account.amount),
            a if a == SRM_MINT => (parsed_token_account.amount, 0),
            _ => {
                msg!("Invalid mint for discount token acccount.");
                return Err(ProgramError::InvalidArgument);
            }
        };
        Ok(Self::from_srm_and_msrm_balances(srm_held, msrm_held))
    }

    pub fn taker_rate(self) -> u64 {
        match self {
            FeeTier::Base => (22 << 32) / 10_000,
            FeeTier::Srm2 => (20 << 32) / 10_000,
            FeeTier::Srm3 => (18 << 32) / 10_000,
            FeeTier::Srm4 => (16 << 32) / 10_000,
            FeeTier::Srm5 => (14 << 32) / 10_000,
            FeeTier::Srm6 => (12 << 32) / 10_000,
            FeeTier::MSrm => (10 << 32) / 10_000,
        }
    }

    pub fn maker_rebate(self, quote_qty: u64) -> u64 {
        let rate = match self {
            FeeTier::MSrm => (5 << 32) / 10_000,
            _ => (3 << 32) / 10_000,
        };
        fp32_mul(quote_qty, rate)
    }

    pub fn remove_taker_fee(self, quote_qty: u64) -> u64 {
        let rate = self.taker_rate();
        fp32_div(quote_qty, FP_32_ONE + rate)
    }

    pub fn taker_fee(self, quote_qty: u64) -> u64 {
        let rate = self.taker_rate();
        fp32_mul(quote_qty, rate)
    }
}
#[doc(hidden)]
#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct CallBackInfo {
    pub user_account: Pubkey,
    pub fee_tier: FeeTier,
}
