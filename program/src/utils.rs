use crate::error::DexError;
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    program_pack::Pack, pubkey::Pubkey,
};
use spl_token::state::Account;

// Safety verification functions
pub fn check_account_key(
    account: &AccountInfo,
    key: &[u8; 32],
    error: DexError,
) -> Result<(), DexError> {
    if &account.key.to_bytes() != key {
        return Err(error);
    }
    Ok(())
}

pub fn check_account_owner(
    account: &AccountInfo,
    owner: &Pubkey,
    error: DexError,
) -> Result<(), DexError> {
    if account.owner != owner {
        return Err(error);
    }
    Ok(())
}

pub fn check_signer(account: &AccountInfo) -> ProgramResult {
    if !(account.is_signer) {
        return Err(ProgramError::MissingRequiredSignature);
    }
    Ok(())
}

pub(crate) const FP_32_ONE: u64 = 1 << 32;

/// a is fp0, b is fp32 and result is a/b fp0
pub(crate) fn fp32_div(a: u64, b_fp32: u64) -> Option<u64> {
    ((a as u128) << 32)
        .checked_div(b_fp32 as u128)
        .and_then(safe_downcast)
}

/// a is fp0, b is fp32 and result is a*b fp0
pub(crate) fn fp32_mul(a: u64, b_fp32: u64) -> Option<u64> {
    (a as u128)
        .checked_mul(b_fp32 as u128)
        .and_then(|e| safe_downcast(e >> 32))
}

fn safe_downcast(n: u128) -> Option<u64> {
    static BOUND: u128 = u64::MAX as u128;
    if n > BOUND {
        None
    } else {
        Some(n as u64)
    }
}

pub fn check_token_account_owner(account: &AccountInfo, owner: &Pubkey) -> Result<(), DexError> {
    check_account_owner(account, &spl_token::ID, DexError::InvalidStateAccountOwner)?;
    let token_acc = Account::unpack_from_slice(&account.data.borrow()).unwrap();
    if token_acc.owner != *owner {
        msg!("Invalid fee account owner");
        return Err(DexError::InvalidStateAccountOwner);
    }
    Ok(())
}
