use num_traits::FromPrimitive;
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::instruction::DexInstruction;

////////////////////////////////////////////////////////////
// Constants
mod srm_token {
    use solana_program::declare_id;

    declare_id!("SRMuApVNdxXokk5GT7XD5cUUgXMBCoAz2LHeuAoKWRt");
}
mod msrm_token {
    use solana_program::declare_id;

    declare_id!("MSRMcoVyrFxnSgo5uXwone5SKcGhT1KEJMFEkMEWf9L");
}
pub static SRM_MINT: Pubkey = srm_token::ID;
pub static MSRM_MINT: Pubkey = msrm_token::ID;

/// The length in bytes of the callback information in the associated asset agnostic orderbook
pub static CALLBACK_INFO_LEN: u64 = 33;
/// The length in bytes of the callback identifer prefix in the associated asset agnostic orderbook
pub static CALLBACK_ID_LEN: u64 = 32;
/// The most significant bit of the fee tier field in CallBack Info indicates if the transaction is referred
pub static REFERRAL_MASK: u8 = 1 << 7;

#[cfg(not(target = "bpf"))]
pub mod fee_defaults {
    //! This module gives the default values in use for past Serum markets
    #[allow(missing_docs)]
    pub const DEFAULT_FEE_TIER_THRESHOLDS: [u64; 6] = [100, 1_000, 10_000, 100_000, 1_000_000, 1];
    #[allow(missing_docs)]
    pub const DEFAULT_FEE_TIER_TAKER_BPS_RATES: [u64; 7] = [22, 20, 18, 16, 14, 12, 10];
    #[allow(missing_docs)]
    pub const DEFAULT_FEE_TIER_MAKER_BPS_REBATES: [u64; 7] = [3, 3, 3, 3, 3, 3, 5];
}

////////////////////////////////////////////////////////////

#[allow(missing_docs)]
pub mod cancel_order;
#[allow(missing_docs)]
pub mod consume_events;
#[allow(missing_docs)]
pub mod create_market;
#[allow(missing_docs)]
pub mod initialize_account;
#[allow(missing_docs)]
pub mod new_order;
#[allow(missing_docs)]
pub mod settle;
#[allow(missing_docs)]
pub mod sweep_fees;

#[allow(missing_docs)]
pub mod close_account;
#[allow(missing_docs)]
pub mod close_market;

pub struct Processor {}

// We add an offset larger than 1 to keep the instruction's internal arguments aligned
pub(crate) const INSTRUCTION_TAG_OFFSET: usize = 8;

impl Processor {
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        msg!("Beginning processing");
        let instruction_tag = FromPrimitive::from_u8(instruction_data[0])
            .ok_or(ProgramError::InvalidInstructionData)?;
        let instruction_data = &instruction_data[INSTRUCTION_TAG_OFFSET..];

        match instruction_tag {
            DexInstruction::CreateMarket => {
                msg!("Instruction: Create Market");
                create_market::process(program_id, accounts, instruction_data)?;
            }
            DexInstruction::NewOrder => {
                msg!("Instruction: New Order");
                new_order::process(program_id, accounts, instruction_data)?;
            }
            DexInstruction::ConsumeEvents => {
                msg!("Instruction: Consume Events");
                consume_events::process(program_id, accounts, instruction_data)?;
            }
            DexInstruction::CancelOrder => {
                msg!("Instruction: Cancel Order");
                cancel_order::process(program_id, accounts, instruction_data)?;
            }
            DexInstruction::Settle => {
                msg!("Instruction: Settle");
                settle::process(program_id, accounts)?;
            }
            DexInstruction::InitializeAccount => {
                msg!("Instruction: Initialize account");
                initialize_account::process(program_id, accounts, instruction_data)?;
            }
            DexInstruction::SweepFees => {
                msg!("Instruction: Sweep fees");
                sweep_fees::process(program_id, accounts)?;
            }
            DexInstruction::CloseAccount => {
                msg!("Instruction: Close Account");
                close_account::process(program_id, accounts)?;
            }
            DexInstruction::CloseMarket => {
                msg!("Instruction: Close Market");
                close_market::process(program_id, accounts)?
            }
        }
        Ok(())
    }
}
