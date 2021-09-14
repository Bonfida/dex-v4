use borsh::BorshDeserialize;
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

pub static CALLBACK_INFO_LEN: u64 = 33;
pub static CALLBACK_ID_LEN: u64 = 32;

////////////////////////////////////////////////////////////

pub mod cancel_order;
pub mod consume_events;
pub mod create_market;
pub mod initialize_account;
pub mod new_order;
pub mod settle;
pub mod sweep_fees;

pub struct Processor {}

impl Processor {
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        msg!("Beginning processing");
        let instruction = DexInstruction::try_from_slice(instruction_data)
            .map_err(|_| ProgramError::InvalidInstructionData)?;
        msg!("Instruction unpacked");

        match instruction {
            DexInstruction::CreateMarket(params) => {
                msg!("Instruction: Create Market");
                create_market::process(program_id, accounts, params)?;
            }
            DexInstruction::NewOrder(params) => {
                msg!("Instruction: New Order");
                new_order::process(program_id, accounts, params)?;
            }
            DexInstruction::ConsumeEvents(params) => {
                msg!("Instruction: Consume Events");
                consume_events::process(program_id, accounts, params)?;
            }
            DexInstruction::CancelOrder(params) => {
                msg!("Instruction: Cancel Order");
                cancel_order::process(program_id, accounts, params)?;
            }
            DexInstruction::Settle(params) => {
                msg!("Instruction: Settle");
                settle::process(program_id, accounts, params)?;
            }
            DexInstruction::InitializeAccount(params) => {
                msg!("Instruction: Initialize account");
                initialize_account::process(program_id, accounts, params)?;
            }
            DexInstruction::SweepFees => {
                msg!("Instruction: Initialize account");
                sweep_fees::process(program_id, accounts)?;
            }
        }
        Ok(())
    }
}
