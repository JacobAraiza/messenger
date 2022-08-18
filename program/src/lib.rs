use anchor_lang::{prelude::*, InstructionData};
use solana_program::instruction::Instruction;

declare_id!("2Ls5MquEmp42AXBxKXX3a9Gu54aPYYVC19tV7RCMKsTt");

pub fn send_direct_mesage(
    sender: Pubkey, 
    receiver: Pubkey, 
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
            AccountMeta::new_readonly(receiver, false),
            AccountMeta::new(chat_pda, false),
            AccountMeta::new(message_pda, false),
            AccountMeta::new(solana_program::system_program::ID, false),
        ],
    )
}

#[program]
pub mod mesenger {
    use super::*;

    #[allow(unused_variables)] // `message_seed` used in `init` of `SendDirectMessage`
    pub fn send_direct_message(context: Context<SendDirectMessage>, message_seed: Vec<u8>, encrypted_text: Vec<u8>) -> Result<()> {
        if encrypted_text.len() > MAX_STRING_BYTES {
            return err!(ChatError::MessageTextTooLarge);
        }

        if context.accounts.chat.initialised {
            // Check participents match the existing direct chat
            let sender_junior = context.accounts.chat.junior == context.accounts.sender.key() && context.accounts.chat.senior == context.accounts.receiver.key();
            let sender_senior = context.accounts.chat.senior == context.accounts.sender.key() && context.accounts.chat.junior == context.accounts.receiver.key();
            if !(sender_junior || sender_senior) {
                return err!(ChatError::DirectChatParticipentMismatch);
            }
        } else {
            // First message in chat
            context.accounts.chat.junior = std::cmp::min(context.accounts.sender.key(), context.accounts.receiver.key());
            context.accounts.chat.senior = std::cmp::max(context.accounts.sender.key(), context.accounts.receiver.key());
            context.accounts.chat.initialised = true;
        }

        // Set message data
        let direction = if context.accounts.sender.key() == context.accounts.chat.senior {
            Direction::SeniorToJunior
        } else {
            Direction::JuniorToSenior
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
    MessageTextTooLarge,
    #[msg("Direct chat already exists: cannot reinitialize")]
    DirectChatAlreadyExists,
    #[msg("Direct chat does not matcher sender and receiver")]
    DirectChatParticipentMismatch,
}

#[account]
pub struct DirectChat {
    pub initialised: bool,
    pub junior: Pubkey,
    pub senior: Pubkey,
    pub last_message: Option<Pubkey>,
}

// https://book.anchor-lang.com/anchor_references/space.html
const DIRECT_CHAT_SIZE: usize = 1 + 32 + 32 + 1 + 32;

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
    JuniorToSenior,
    SeniorToJunior,
}

#[derive(Accounts)]
#[instruction(message_seed: Vec<u8>)]
pub struct SendDirectMessage<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,
    pub receiver: AccountInfo<'info>,
    #[account(
        init_if_needed, 
        payer = sender, 
        owner = *program_id,
        seeds = [
            std::cmp::min(sender.key().as_ref(), receiver.key().as_ref()),
            std::cmp::max(sender.key().as_ref(), receiver.key().as_ref())
        ],
        bump,
        space = 8 + DIRECT_CHAT_SIZE
    )]
    pub chat: Account<'info, DirectChat>,
    #[account(
        init_if_needed, 
        payer = sender, 
        owner = *program_id,
        seeds = [message_seed.as_ref()],
        bump,
        space = 8 + MESSAGE_MAX_SIZE
    )]
    pub message: Account<'info, Message>,
    pub system_program: Program<'info, System>,
}

pub fn direct_chat_pda(sender: &Pubkey, receiver: &Pubkey) -> Pubkey {
    let chat_seed = direct_message_seed(sender, receiver);
    let (chat_pda, _chat_bump) = Pubkey::find_program_address(&chat_seed, &ID);
    chat_pda
}

pub fn direct_message_seed<'a>(sender: &'a Pubkey, receiver: &'a Pubkey) -> [&'a [u8]; 2] {
    if sender < receiver {
        [sender.as_ref(), receiver.as_ref()]
    } else {
        [receiver.as_ref(), sender.as_ref()]
    }
}
