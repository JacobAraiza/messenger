use anchor_lang::{prelude::*, InstructionData};
use solana_program::instruction::Instruction;

#[account]
pub struct Mailbox {
    pub inbox: Option<Pubkey>,
}

#[account]
pub struct Message {
    pub from: Pubkey,
    pub inbox: Option<Pubkey>,
    pub ciphertext: Vec<u8>,
}

// https://book.anchor-lang.com/anchor_references/space.html
const DIRECT_CHAT_SIZE: usize = 1 + 32;
const MESSAGE_MAX_SIZE: usize = 32 + 1 + 32 + 4 + MAX_STRING_BYTES;
const MAX_STRING_BYTES: usize = 255;

#[derive(Accounts)]
#[instruction(message_seed: Vec<u8>)]
pub struct SendDirectMessage<'info> {
    #[account(mut)]
    pub from: Signer<'info>,
    pub to: AccountInfo<'info>,
    #[account(
        init_if_needed, 
        payer = from, 
        owner = *program_id,
        seeds = [to.key().as_ref()],
        bump,
        space = 8 + DIRECT_CHAT_SIZE
    )]
    pub mailbox: Account<'info, Mailbox>,
    #[account(
        init_if_needed, 
        payer = from, 
        owner = *program_id,
        seeds = [message_seed.as_ref()],
        bump,
        space = 8 + MESSAGE_MAX_SIZE
    )]
    pub message: Account<'info, Message>,
    pub system_program: Program<'info, System>,
}

#[program]
pub mod mesenger {
    use super::*;

    #[allow(unused_variables)] // `message_seed` used in `init` of `SendDirectMessage`
    pub fn send_direct_message(
        context: Context<SendDirectMessage>,
        message_seed: Vec<u8>,
        ciphertext: Vec<u8>,
    ) -> Result<()> {
        if ciphertext.len() > MAX_STRING_BYTES {
            return err!(ChatError::MessageTextTooLarge);
        }

        // Set message data
        context.accounts.message.from = context.accounts.from.key();
        context.accounts.message.ciphertext = ciphertext;

        // Add message to inbox
        context.accounts.message.inbox = context.accounts.mailbox.inbox;
        context.accounts.mailbox.inbox = Some(context.accounts.message.key());

        Ok(())
    }
}

#[error_code]
pub enum ChatError {
    #[msg("Message text is too many bytes (maximum of 255 bytes)")]
    MessageTextTooLarge,
}

pub fn mailbox_pda(owner: &Pubkey) -> Pubkey {
    let seed = [owner.as_ref()];
    let (pda, _chat_bump) = Pubkey::find_program_address(&seed, &ID);
    pda
}

pub fn send_direct_mesage(
    sender: Pubkey,
    receiver: Pubkey,
    message_seed: Vec<u8>,
    message_pda: Pubkey,
    ciphertext: Vec<u8>,
) -> Instruction {
    let instruction = instruction::SendDirectMessage {
        message_seed,
        ciphertext,
    };
    Instruction::new_with_bytes(
        ID,
        &instruction.data(),
        vec![
            AccountMeta::new(sender, true),
            AccountMeta::new_readonly(receiver, false),
            AccountMeta::new(mailbox_pda(&receiver), false),
            AccountMeta::new(message_pda, false),
            AccountMeta::new(solana_program::system_program::ID, false),
        ],
    )
}

declare_id!("2Ls5MquEmp42AXBxKXX3a9Gu54aPYYVC19tV7RCMKsTt");