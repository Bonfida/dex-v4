use borsh::{BorshDeserialize, BorshSerialize};
use bytemuck::{try_cast_slice_mut, try_from_bytes_mut, Pod, Zeroable};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::FromPrimitive;
use solana_program::{
    account_info::AccountInfo, msg, program_error::ProgramError, program_pack::Pack, pubkey::Pubkey,
};
use std::{cell::RefMut, mem::size_of};

use crate::{
    error::DexError,
    processor::{MSRM_MINT, REFERRAL_MASK, SRM_MINT},
    utils::{fp32_div, fp32_mul, FP_32_ONE},
};

#[derive(Clone, Debug, PartialEq, Copy)]
#[allow(missing_docs)]
#[repr(u64)]
pub enum AccountTag {
    Uninitialized,
    DexState,
    UserAccount,
    Closed,
}

#[derive(Clone, Copy, PartialEq, FromPrimitive, ToPrimitive)]
#[repr(u8)]
#[allow(missing_docs)]
pub enum Side {
    Bid,
    Ask,
}

/// This enum describes different supported behaviors for handling self trading scenarios
#[derive(PartialEq, Clone, Copy)]
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
    pub base_mint: Pubkey,
    /// The mint key of the quote token
    pub quote_mint: Pubkey,
    /// The SPL token account holding the market's base tokens
    pub base_vault: Pubkey,
    /// The SPL token account holding the market's quote tokens
    pub quote_vault: Pubkey,
    /// The asset agnostic orderbook address
    pub orderbook: Pubkey,
    /// The market admin which can recuperate all transaction fees
    pub admin: Pubkey,
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
    pub signer_nonce: u8,
    /// Fee type (e.g. default or stable)
    pub fee_type: u8,
    /// Padding
    pub _padding: [u8; 6],
}

impl DexState {
    /// Size in bytes of the dex state object
    pub const LEN: usize = size_of::<Self>();
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
            try_from_bytes_mut::<Self>(&mut s[0..Self::LEN]).unwrap()
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
    pub market: Pubkey,
    /// The user account owner's wallet
    pub owner: Pubkey,
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

/// Represents and order in the user account. The client id offers an alias which can be used off-chain to map custom ids to an actual order id.
#[derive(Copy, Clone, Pod, Zeroable, PartialEq, Debug)]
#[repr(C)]
pub struct Order {
    /// The raw order id
    pub id: u128,
    /// The client-defined order id. Care should be taken off-chain to only create new orders with new client_ids.
    pub client_id: u128,
}

impl Order {
    /// The length in bytes of the order's binary representation
    pub const LEN: usize = std::mem::size_of::<Self>();
}

#[allow(missing_docs)]
pub struct UserAccount<'a> {
    pub header: &'a mut UserAccountHeader,
    orders: &'a mut [Order],
}

impl UserAccountHeader {
    pub const LEN: usize = std::mem::size_of::<Self>();
    #[doc(hidden)]
    pub fn new(market: &Pubkey, owner: &Pubkey) -> Self {
        Self {
            tag: AccountTag::UserAccount as u64,
            market: *market,
            owner: *owner,
            base_token_free: 0,
            base_token_locked: 0,
            quote_token_free: 0,
            quote_token_locked: 0,
            number_of_orders: 0,
            accumulated_rebates: 0,
            _padding: 0,
            accumulated_maker_quote_volume: 0,
            accumulated_maker_base_volume: 0,
            accumulated_taker_quote_volume: 0,
            accumulated_taker_base_volume: 0,
        }
    }
}

impl<'a> UserAccount<'a> {
    #[allow(missing_docs)]
    pub fn compute_allocation_size(order_capacity: usize) -> Result<usize, DexError> {
        order_capacity
            .checked_mul(Order::LEN)
            .and_then(|n| n.checked_add(UserAccountHeader::LEN))
            .ok_or(DexError::NumericalOverflow)
    }

    #[allow(missing_docs)]
    pub fn from_buffer(buf: &'a mut [u8]) -> Result<Self, ProgramError> {
        let user_acc = UserAccount::from_buffer_unchecked(buf).unwrap();
        if user_acc.header.tag != AccountTag::UserAccount as u64 {
            return Err(ProgramError::InvalidAccountData);
        };
        Ok(user_acc)
    }

    #[allow(missing_docs)]
    pub fn from_buffer_unchecked(buf: &'a mut [u8]) -> Result<Self, ProgramError> {
        let (hd, tl) = buf.split_at_mut(UserAccountHeader::LEN);
        let header: &mut UserAccountHeader = try_from_bytes_mut(hd).unwrap();
        let orders = try_cast_slice_mut(tl).unwrap();

        Ok(Self { header, orders })
    }
}

impl<'a> UserAccount<'a> {
    #[allow(missing_docs)]
    pub fn read_order(&self, order_index: usize) -> Result<Order, DexError> {
        if order_index >= self.header.number_of_orders as usize {
            return Err(DexError::InvalidOrderIndex);
        }
        Ok(self.orders[order_index])
    }

    #[allow(missing_docs)]
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

    #[allow(missing_docs)]
    pub fn add_order(&mut self, order: Order) -> Result<(), DexError> {
        let slot = self
            .orders
            .get_mut(self.header.number_of_orders as usize)
            .ok_or(DexError::UserAccountFull)?;
        *slot = order;
        self.header.number_of_orders += 1;
        Ok(())
    }

    #[allow(missing_docs)]
    pub fn find_order_index(&self, order_id: u128) -> Result<usize, DexError> {
        let res = self
            .orders
            .iter()
            .enumerate()
            .find(|(_, b)| b.id == order_id)
            .ok_or(DexError::OrderNotFound)?
            .0;
        Ok(res)
    }

    #[allow(missing_docs)]
    pub fn find_order_id_by_client_id(&self, client_order_id: u128) -> Result<u128, DexError> {
        let res = self
            .orders
            .iter()
            .find(|b| b.client_id == client_order_id)
            .ok_or(DexError::OrderNotFound)?
            .id;
        Ok(res)
    }
}

#[doc(hidden)]
#[derive(BorshDeserialize, BorshSerialize, Debug, Clone, Copy)]
pub enum MarketFeeType {
    Default,
    Stable,
}

#[doc(hidden)]
#[derive(BorshDeserialize, BorshSerialize, Debug, Clone, Copy, FromPrimitive, PartialEq)]
pub enum FeeTier {
    Base,
    Srm2,
    Srm3,
    Srm4,
    Srm5,
    Srm6,
    MSrm,
    Stable,
}

#[doc(hidden)]
impl FeeTier {
    pub fn from_srm_and_msrm_balances(
        dex_state: &DexState,
        srm_held: u64,
        msrm_held: u64,
    ) -> FeeTier {
        let one_srm = 1_000_000;

        if dex_state.fee_type == MarketFeeType::Stable as u8 {
            return FeeTier::Stable;
        }

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

    pub fn from_u8(tag: u8) -> (Self, bool) {
        let is_referred = (tag & REFERRAL_MASK) != 0;
        let fee_tier = <Self as FromPrimitive>::from_u8(tag & (!REFERRAL_MASK)).unwrap();
        (fee_tier, is_referred)
    }

    pub fn get(
        dex_state: &DexState,
        account_data: &[u8],
        expected_owner: &Pubkey,
    ) -> Result<Self, ProgramError> {
        let parsed_token_account = spl_token::state::Account::unpack(account_data)?;
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
        Ok(Self::from_srm_and_msrm_balances(
            dex_state, srm_held, msrm_held,
        ))
    }

    pub fn taker_rate(self) -> u64 {
        static TAKER_RATES: [u64; 8] = [
            (40 << 32) / 100_000,
            (39 << 32) / 100_000,
            (38 << 32) / 100_000,
            (36 << 32) / 100_000,
            (34 << 32) / 100_000,
            (32 << 32) / 100_000,
            (30 << 32) / 100_000,
            (10 << 32) / 100_000,
        ];
        TAKER_RATES[self as usize]
    }

    pub fn maker_rate(self) -> u64 {
        0
    }

    pub fn maker_rebate(self, _quote_qty: u64) -> u64 {
        0
    }

    pub fn remove_taker_fee(self, quote_qty: u64) -> u64 {
        let rate = self.taker_rate();
        fp32_div(quote_qty, FP_32_ONE + rate).unwrap()
    }

    pub fn taker_fee(self, quote_qty: u64) -> u64 {
        let rate = self.taker_rate();
        fp32_mul(quote_qty, rate).unwrap()
    }

    pub fn referral_rate(self) -> u64 {
        let taker_rate = self.taker_rate();
        let min_maker_rebate = Self::Base.maker_rate();
        taker_rate.saturating_sub(min_maker_rebate) / 5
    }

    pub fn referral_fee(self, quote_qty: u64) -> u64 {
        let rate = self.referral_rate();
        fp32_mul(quote_qty, rate).unwrap()
    }
}
#[doc(hidden)]
#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct CallBackInfo {
    pub user_account: Pubkey,
    pub fee_tier: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_account() {
        let order_capacity = 100;

        let market_key = Pubkey::new_unique();
        let user = Pubkey::new_unique();

        let user_account_size = UserAccount::compute_allocation_size(order_capacity).unwrap();
        let mut user_account_data = vec![0; user_account_size];
        let mut user_account = UserAccount::from_buffer_unchecked(&mut user_account_data).unwrap();
        *user_account.header = UserAccountHeader::new(&market_key, &user);

        for i in 1..((order_capacity + 1) as u128) {
            user_account
                .add_order(Order {
                    id: i << 1,
                    client_id: (i << 1) + 1,
                })
                .unwrap();
        }

        assert!(user_account
            .add_order(Order {
                id: 0,
                client_id: 1,
            })
            .is_err());
        for i in 1..(order_capacity + 1) as u128 {
            let order_index = i as usize - 1;
            let read_order = user_account.read_order(order_index).unwrap();
            let expected_order = Order {
                id: i << 1,
                client_id: (i << 1) + 1,
            };
            assert_eq!(read_order, expected_order);
            assert_eq!(
                order_index,
                user_account.find_order_index(expected_order.id).unwrap()
            );
            assert_eq!(
                expected_order.id,
                user_account
                    .find_order_id_by_client_id(expected_order.client_id)
                    .unwrap()
            );
        }
    }

    #[test]
    fn test_fee_tiers() {
        assert_eq!(FeeTier::Base.taker_rate(), (40 << 32) / 100_000);
        assert_eq!(FeeTier::Srm2.taker_rate(), (39 << 32) / 100_000);
        assert_eq!(FeeTier::Srm3.taker_rate(), (38 << 32) / 100_000);
        assert_eq!(FeeTier::Srm4.taker_rate(), (36 << 32) / 100_000);
        assert_eq!(FeeTier::Srm5.taker_rate(), (34 << 32) / 100_000);
        assert_eq!(FeeTier::Srm6.taker_rate(), (32 << 32) / 100_000);
        assert_eq!(FeeTier::MSrm.taker_rate(), (30 << 32) / 100_000);
        assert_eq!(FeeTier::Stable.taker_rate(), (10 << 32) / 100_000);

        let mut dummy_dex_state = vec![0; DexState::LEN];
        let dex_state = bytemuck::from_bytes_mut::<DexState>(&mut dummy_dex_state);
        dex_state.fee_type = MarketFeeType::Default as u8;
        let one_srm = 1_000_000;
        assert_eq!(
            FeeTier::from_srm_and_msrm_balances(dex_state, 0, 1) as u8,
            FeeTier::MSrm as u8
        );
        assert_eq!(
            FeeTier::from_srm_and_msrm_balances(dex_state, 0, 0) as u8,
            FeeTier::Base as u8
        );
        assert_eq!(
            FeeTier::from_srm_and_msrm_balances(dex_state, 100 * one_srm, 0) as u8,
            FeeTier::Srm2 as u8
        );
        assert_eq!(
            FeeTier::from_srm_and_msrm_balances(dex_state, 1_000 * one_srm, 0) as u8,
            FeeTier::Srm3 as u8
        );
        assert_eq!(
            FeeTier::from_srm_and_msrm_balances(dex_state, 10_000 * one_srm, 0) as u8,
            FeeTier::Srm4 as u8
        );
        assert_eq!(
            FeeTier::from_srm_and_msrm_balances(dex_state, 100_000 * one_srm, 0) as u8,
            FeeTier::Srm5 as u8
        );
        assert_eq!(
            FeeTier::from_srm_and_msrm_balances(dex_state, 1_000_000 * one_srm, 0) as u8,
            FeeTier::Srm6 as u8
        );

        dex_state.fee_type = MarketFeeType::Stable as u8;

        assert_eq!(
            FeeTier::from_srm_and_msrm_balances(dex_state, 1_000 * one_srm, 0) as u8,
            FeeTier::Stable as u8
        );
    }

    #[test]
    fn test_fee_tiers_sec() {
        let mut dummy_token_account = vec![0; spl_token::state::Account::LEN];
        let owner = Pubkey::new_unique();
        let mut account = spl_token::state::Account {
            mint: Pubkey::new_unique(),
            owner,
            amount: 1_000_000_000,
            state: spl_token::state::AccountState::Initialized,
            ..Default::default()
        };
        account.pack_into_slice(&mut dummy_token_account);

        let mut dummy_dex_state = vec![0; DexState::LEN];
        let dex_state = bytemuck::from_bytes_mut::<DexState>(&mut dummy_dex_state);
        dex_state.fee_type = MarketFeeType::Default as u8;

        assert!(FeeTier::get(dex_state, &dummy_token_account, &Pubkey::new_unique()).is_err());
        assert!(FeeTier::get(dex_state, &dummy_token_account, &owner).is_err());
        account.mint = crate::constants::SRM_MINT;
        account.pack_into_slice(&mut dummy_token_account);
        assert!(FeeTier::get(dex_state, &dummy_token_account, &Pubkey::new_unique()).is_err());
        assert!(FeeTier::get(dex_state, &dummy_token_account, &owner).is_ok());
    }
}
