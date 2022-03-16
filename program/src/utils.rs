use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    pubkey::Pubkey,
    bpf_loader_upgradeable::{self, UpgradeableLoaderState}
};

use bincode::{self};
use crate::error::DexError;

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

pub fn check_program_upgrade_authority(
    program: &AccountInfo,
    program_data: &AccountInfo,
    upgrade_authority: &AccountInfo,
) -> ProgramResult {
    check_account_owner(program, &bpf_loader_upgradeable::ID, DexError::InvalidStateAccountOwner)?;

    let program_account_state = bincode::deserialize(&program.data.borrow()).map_err(|_| {
        ProgramError::InvalidAccountData
    })?;

    let program_data_state = bincode::deserialize(&program_data.data.borrow()).map_err(|_| {
        ProgramError::InvalidAccountData
    })?;

    if let UpgradeableLoaderState::Program { programdata_address } = program_account_state {
        if programdata_address != *program_data.key {
            return Err(ProgramError::InvalidAccountData)
        }
    }
    else {
        return Err(ProgramError::InvalidAccountData)
    }

    if let UpgradeableLoaderState::ProgramData { slot: _, upgrade_authority_address } = program_data_state {
        match upgrade_authority_address {
            Some(address) => {
                if address != *upgrade_authority.key {
                    return Err(ProgramError::InvalidAccountData)
                }
            },
            None => return Err(ProgramError::InvalidAccountData)
        }
    }
    else {
        return Err(ProgramError::InvalidAccountData)
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
