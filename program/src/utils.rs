use crate::error::DexError;
use mpl_token_metadata::{
    pda::find_metadata_account,
    state::{Creator, Metadata},
};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    pubkey::Pubkey,
};

// Safety verification functions
pub fn check_account_key(
    account: &AccountInfo,
    key: &Pubkey,
    error: DexError,
) -> Result<(), DexError> {
    if account.key != key {
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

pub fn check_metadata_account(account: &AccountInfo, mint: &Pubkey) -> ProgramResult {
    let expected = find_metadata_account(mint).0;
    check_account_key(account, &expected, DexError::InvalidMetadataKey)?;
    if account.data_len() != 0 {
        check_account_owner(
            account,
            &mpl_token_metadata::ID,
            DexError::InvalidMetadataOwner,
        )?;
    }

    Ok(())
}

#[allow(dead_code)]
pub fn get_verified_creators(account: &AccountInfo) -> Option<Vec<Creator>> {
    let metadata = Metadata::from_account_info(account).unwrap();
    let creators = metadata.data.creators;

    if let Some(creators) = creators {
        return Some(
            creators
                .into_iter()
                .filter(|creator| creator.verified)
                .collect(),
        );
    }

    None
}

pub fn verify_metadata(creators: &[Creator]) -> ProgramResult {
    let sum: u8 = creators.iter().map(|x| x.share).sum();
    if sum != 100 {
        msg!("Invalid metadata shares - received {}", sum);
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}
