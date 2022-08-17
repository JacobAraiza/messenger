use anchor_lang::{prelude::*, InstructionData};
use solana_program::instruction::Instruction;

declare_id!("CwrqeMj2U8tFr1Rhkgwc84tpAsqbt9pTt2a4taoTADPr");

pub fn initialize_direct_chat(
    initiator: Pubkey,
    reciprocator: Pubkey,
    chat_pda: Pubkey
) -> Instruction {
    let instruction = instruction::InitialiseDirectChat {};
    Instruction::new_with_bytes(
        ID,
        &instruction.data(),
        vec![
            AccountMeta::new(initiator, true),
            AccountMeta::new_readonly(reciprocator, false),
            AccountMeta::new(chat_pda, false),
            AccountMeta::new(solana_program::system_program::ID, false),
        ],
    )
}

pub fn send_direct_mesage(
    sender: Pubkey, 
    chat_pda: Pubkey, 
    message_seed: Vec<u8>, 
    message_pda: Pubkey, 
    encrypted_text: Vec<u8>
) -> Instruction {
    let instruction = instruction::SendDirectMessage {
        message_seed,
        encrypted_text
    };
    Instruction::new_with_bytes(
        ID,
        &instruction.data(),
        vec![
            AccountMeta::new(sender, true),
            AccountMeta::new(chat_pda, false),
            AccountMeta::new(message_pda, false),
            AccountMeta::new(solana_program::system_program::ID, false),
        ],
    )
}

#[program]
pub mod mesenger {

    use super::*;

    pub fn initialise_direct_chat(context: Context<StartDirectChat>) -> Result<()> {
        context.accounts.chat.initiator = context.accounts.initiator.key();
        context.accounts.chat.reciprocator = context.accounts.reciprocator.key();
        Ok(())
    }

    #[allow(unused_variables)] // message seed used in `init` of SendDirectMessage
    pub fn send_direct_message(context: Context<SendDirectMessage>, message_seed: Vec<u8>, encrypted_text: Vec<u8>) -> Result<()> {
        if encrypted_text.len() > MAX_STRING_BYTES {
            return err!(ChatError::MessageTextTooLarge);
        }

        // Set message data
        let direction = if context.accounts.sender.key() == context.accounts.chat.initiator {
            Direction::InitiatorToReciprocator
        } else {
            Direction::ReciprocatorToInitiator
        };
        context.accounts.message.direction = direction;
        context.accounts.message.encrypted_text = encrypted_text;

        // Add message to end of list
        context.accounts.message.previous_message = context.accounts.chat.last_message;
        context.accounts.chat.last_message = Some(context.accounts.message.key());

        Ok(())
    }
}

#[error_code]
pub enum ChatError {
    #[msg("Message text is too many bytes (maximum of 255 bytes)")]
    MessageTextTooLarge
}

#[derive(Accounts)]
pub struct StartDirectChat<'info> {
    #[account(mut)]
    pub initiator: Signer<'info>,
    pub reciprocator: AccountInfo<'info>,
    #[account(
        init, 
        payer = initiator, 
        owner = *program_id,
        seeds = [
            std::cmp::min(initiator.key().as_ref(), reciprocator.key().as_ref()),
            std::cmp::max(initiator.key().as_ref(), reciprocator.key().as_ref())
        ],
        bump,
        space = 8 + DIRECT_CHAT_SIZE
    )]
    pub chat: Account<'info, DirectChat>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct DirectChat {
    pub initiator: Pubkey,
    pub reciprocator: Pubkey,
    pub last_message: Option<Pubkey>,
}

// https://book.anchor-lang.com/anchor_references/space.html
const DIRECT_CHAT_SIZE: usize = 32 + 32 + 1 + 32;

#[account]
pub struct Message {
    pub direction: Direction,
    pub previous_message: Option<Pubkey>,
    pub encrypted_text: Vec<u8>,
}

// https://book.anchor-lang.com/anchor_references/space.html
const MESSAGE_MAX_SIZE: usize = 1 + 1 + 32 + 4 + MAX_STRING_BYTES;

const MAX_STRING_BYTES: usize = 255;

#[derive(AnchorDeserialize, AnchorSerialize, Clone, Eq, PartialEq, Debug)]
pub enum Direction {
    InitiatorToReciprocator,
    ReciprocatorToInitiator,
}

#[derive(Accounts)]
#[instruction(message_seed: Vec<u8>)]
pub struct SendDirectMessage<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,
    #[account(
        mut,
        owner = *program_id,
        constraint = (sender.key() == chat.initiator || sender.key() == chat.reciprocator)
    )]
    pub chat: Account<'info, DirectChat>,
    #[account(
        init, 
        payer = sender, 
        owner = *program_id,
        seeds = [message_seed.as_ref()],
        bump,
        space = 8 + MESSAGE_MAX_SIZE
    )]
    pub message: Account<'info, Message>,
    pub system_program: Program<'info, System>,
}

pub fn direct_message_seed<'a>(initiator: &'a Pubkey, recipricator: &'a Pubkey) -> [&'a [u8]; 2] {
    if initiator < recipricator {
        [initiator.as_ref(), recipricator.as_ref()]
    } else {
        [recipricator.as_ref(), initiator.as_ref()]
    }
}
