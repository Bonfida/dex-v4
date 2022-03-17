use num_traits::FromPrimitive;
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    pubkey::{Pubkey}
};

use crate::instruction_auto::DexInstruction;

////////////////////////////////////////////////////////////
// Constants
pub static SRM_MINT: Pubkey = solana_program::pubkey!("SRMuApVNdxXokk5GT7XD5cUUgXMBCoAz2LHeuAoKWRt");
pub static MSRM_MINT: Pubkey = solana_program::pubkey!("MSRMcoVyrFxnSgo5uXwone5SKcGhT1KEJMFEkMEWf9L");

/// The sweep authority for the DEX program
pub static SWEEP_AUTHORITY: Pubkey = solana_program::pubkey!("DjXsn34uz8hnC4KLiSkEVNmzqX5ZFP2Q7aErTBH8LWxe");

/// The length in bytes of the callback information in the associated asset agnostic orderbook
pub static CALLBACK_INFO_LEN: u64 = 33;
/// The length in bytes of the callback identifer prefix in the associated asset agnostic orderbook
pub static CALLBACK_ID_LEN: u64 = 32;
/// The most significant bit of the fee tier field in CallBack Info indicates if the transaction is referred
pub static REFERRAL_MASK: u8 = 1 << 7;

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
pub mod swap;
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
            DexInstruction::Swap => {
                msg!("Instruction: Swap");
                swap::process(program_id, accounts, instruction_data)?;
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
