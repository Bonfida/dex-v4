use borsh::{BorshDeserialize, BorshSerialize};
use num_derive::{FromPrimitive, ToPrimitive};
use solana_program::{
    account_info::AccountInfo,
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};
use std::{cell::RefCell, convert::TryInto, rc::Rc};

use crate::{
    error::DexError,
    processor::{MSRM_MINT, SRM_MINT},
    utils::{fp32_div, fp32_mul, FP_32_ONE},
};

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, PartialEq)]
#[allow(missing_docs)]
pub enum AccountTag {
    Uninitialized,
    DexState,
    UserAccount,
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Copy, PartialEq, FromPrimitive, ToPrimitive)]
#[repr(u8)]
#[allow(missing_docs)]
pub enum Side {
    Bid,
    Ask,
}

impl Side {
    pub fn opposite(&self) -> Self {
        match self {
            Side::Bid => Side::Ask,
            Side::Ask => Side::Bid,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize, Clone, PartialEq)]
pub enum SelfTradeBehavior {
    DecrementTake,
    CancelProvide,
    AbortTransaction,
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct DexState {
    pub tag: AccountTag,
    pub signer_nonce: u8,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub base_vault: Pubkey,
    pub quote_vault: Pubkey,
    pub orderbook: Pubkey,
    pub aaob_program: Pubkey,
    pub admin: Pubkey,
    pub creation_timestamp: i64,
    pub base_volume: u64,
    pub quote_volume: u64, //TODO metrics (volume quote, volume base)
    pub accumulated_fees: u64,
}

impl DexState {
    pub(crate) fn check(self) -> Result<Self, ProgramError> {
        if self.tag != AccountTag::DexState {
            return Err(ProgramError::InvalidAccountData);
        };
        Ok(self)
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct UserAccountHeader {
    pub tag: AccountTag,
    pub market: Pubkey,
    pub owner: Pubkey,
    pub base_token_free: u64,
    pub base_token_locked: u64,
    pub quote_token_free: u64,
    pub quote_token_locked: u64,
    pub number_of_orders: u32,
    pub accumulated_rebates: u64,
}

impl Sealed for UserAccountHeader {}

impl Pack for UserAccountHeader {
    const LEN: usize = 109;

    fn pack_into_slice(&self, mut dst: &mut [u8]) {
        self.serialize(&mut dst).unwrap()
    }

    fn unpack_from_slice(mut src: &[u8]) -> Result<Self, ProgramError> {
        UserAccountHeader::deserialize(&mut src).map_err(|_| ProgramError::InvalidAccountData)
    }
}

impl IsInitialized for UserAccountHeader {
    fn is_initialized(&self) -> bool {
        self.tag == AccountTag::UserAccount
    }
}

pub struct UserAccount<'a> {
    pub header: UserAccountHeader,
    data: Rc<RefCell<&'a mut [u8]>>,
}

impl<'a> UserAccount<'a> {
    pub fn new(account: &AccountInfo<'a>, header: UserAccountHeader) -> Self {
        Self {
            header,
            data: Rc::clone(&account.data),
        }
    }
    pub fn parse(account: &AccountInfo<'a>) -> Result<Self, ProgramError> {
        Ok(Self {
            header: UserAccountHeader::unpack(&account.data.borrow())?,
            data: Rc::clone(&account.data),
        })
    }

    pub fn write(&self) {
        self.header.pack_into_slice(&mut self.data.borrow_mut());
    }

    pub fn read_order(&self, order_index: usize) -> Result<u128, DexError> {
        if order_index >= self.header.number_of_orders as usize {
            return Err(DexError::InvalidOrderIndex);
        }
        let offset = UserAccountHeader::LEN + order_index * 16;
        Ok(u128::from_le_bytes(
            self.data.borrow()[offset..offset + 16].try_into().unwrap(),
        ))
    }

    pub fn remove_order(&mut self, order_index: usize) -> Result<(), DexError> {
        if order_index >= self.header.number_of_orders as usize {
            return Err(DexError::InvalidOrderIndex);
        }
        if self.header.number_of_orders - order_index as u32 != 1 {
            let last_order = self.read_order((self.header.number_of_orders - 1) as usize)?;
            let offset = UserAccountHeader::LEN + order_index * 16;
            self.data.borrow_mut()[offset..offset + 16].copy_from_slice(&last_order.to_le_bytes());
        }
        self.header.number_of_orders -= 1;
        Ok(())
    }

    pub fn add_order(&mut self, order: u128) -> Result<(), DexError> {
        let offset = UserAccountHeader::LEN + (self.header.number_of_orders * 16) as usize;
        self.data
            .borrow_mut()
            .get_mut(offset..offset + 16)
            .map(|b| b.copy_from_slice(&order.to_le_bytes()))
            .ok_or(DexError::UserAccountFull)?;
        self.header.number_of_orders += 1;
        Ok(())
    }

    pub fn find_order_index(&self, order_id: u128) -> Result<usize, DexError> {
        let data: &[u8] = &self.data.borrow();
        Ok((UserAccountHeader::LEN..)
            .step_by(16)
            .take(self.header.number_of_orders as usize)
            .map(|offset| u128::from_le_bytes(data[offset..offset + 16].try_into().unwrap()))
            .enumerate()
            .find(|(_, b)| b == &order_id)
            .ok_or(DexError::OrderNotFound)?
            .0)
    }
}

pub trait Order {
    const LEN: usize;
}

impl Order for u128 {
    const LEN: usize = 16;
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Copy)]
pub enum FeeTier {
    Base,
    Srm2,
    Srm3,
    Srm4,
    Srm5,
    Srm6,
    MSrm,
}

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

    pub fn maker_rebate(self, pc_qty: u64) -> u64 {
        let rate = match self {
            FeeTier::MSrm => (5 << 32) / 10_000,
            _ => (3 << 32) / 10_000,
        };
        fp32_mul(pc_qty, rate)
    }

    pub fn remove_taker_fee(self, pc_qty: u64) -> u64 {
        let rate = self.taker_rate();
        fp32_div(pc_qty, FP_32_ONE + rate)
    }

    pub fn taker_fee(self, pc_qty: u64) -> u64 {
        let rate = self.taker_rate();
        fp32_mul(pc_qty, rate)
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct CallBackInfo {
    pub user_account: Pubkey,
    pub fee_tier: FeeTier,
}
