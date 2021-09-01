use borsh::BorshDeserialize;
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::instruction::DexInstruction;

////////////////////////////////////////////////////////////
// Constants

////////////////////////////////////////////////////////////

pub mod cancel_order;
pub mod consume_events;
pub mod create_market;
pub mod new_order;

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
        }
        Ok(())
    }
}
