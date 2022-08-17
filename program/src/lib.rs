use anchor_lang::prelude::*;
use solana_program::instruction::Instruction;

declare_id!("CwrqeMj2U8tFr1Rhkgwc84tpAsqbt9pTt2a4taoTADPr");

pub fn initialize_direct_chat(
    initiator: Pubkey,
    reciprocator: Pubkey,
    chat_pda: Pubkey
) -> Instruction {
    Instruction::new_with_borsh(
        ID,
        &instruction::InitialiseDirectChat {
            reciprocator
        },
        vec![
            AccountMeta::new(initiator, true),
            AccountMeta::new(chat_pda, false),
            AccountMeta::new(solana_program::system_program::ID, false),
        ],
    )
}

pub fn send_direct_mesage(sender: Pubkey, chat_pda: Pubkey, message_pda: Pubkey, encrypted_text: Vec<u8>) -> Instruction {
    Instruction::new_with_borsh(
        ID,
        &instruction::SendDirectMessage {
            encrypted_text
        },
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

    pub fn initialise_direct_chat(
        context: Context<StartDirectChat>,
        reciprocator: Pubkey
    ) -> Result<()> {
        context.accounts.chat.initiator = context.accounts.initiator.key();
        context.accounts.chat.reciprocator = reciprocator;
        Ok(())
    }

    pub fn send_direct_message(context: Context<SendDirectMessage>, encrypted_text: Vec<u8>) -> Result<()> {
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
    #[account(
        init, 
        payer = initiator, 
        owner = *program_id,
        space = 8 + DIRECT_CHAT_SIZE
    )]
    pub chat: Account<'info, DirectChat>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct DirectChat {
    initiator: Pubkey,
    reciprocator: Pubkey,
    last_message: Option<Pubkey>,
}

// https://book.anchor-lang.com/anchor_references/space.html
const DIRECT_CHAT_SIZE: usize = 32 + 32 + 1 + 32;

#[account]
pub struct Message {
    direction: Direction,
    previous_message: Option<Pubkey>,
    encrypted_text: Vec<u8>,
}

// https://book.anchor-lang.com/anchor_references/space.html
const MESSAGE_MAX_SIZE: usize = 1 + 1 + 32 + 4 + MAX_STRING_BYTES;

const MAX_STRING_BYTES: usize = 255;

#[derive(AnchorDeserialize, AnchorSerialize, Clone)]
enum Direction {
    InitiatorToReciprocator,
    ReciprocatorToInitiator,
}

#[derive(Accounts)]
pub struct SendDirectMessage<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,
    #[account(
        owner = *program_id,
        constraint = (sender.key() == chat.initiator || sender.key() == chat.reciprocator)
    )]
    pub chat: Account<'info, DirectChat>,    
    #[account(
        init, 
        payer = sender, 
        owner = *program_id,
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
