use anchor_lang::AccountDeserialize;
use rand::Rng;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig, instruction::Instruction, pubkey::Pubkey,
    signature::read_keypair_file, signature::Keypair, signer::Signer, transaction::Transaction,
};
use structopt::StructOpt;

type Error = Box<dyn std::error::Error>;

mod encrypt;

fn main() {
    let args = Arguments::from_args();
    let client = RpcClient::new_with_commitment(args.url, CommitmentConfig::confirmed());
    let encryption_key = encrypt::SharedKey::new(&args.keypair, &args.to);
    let ciphertext = encryption_key.transmit_key.encrypt(&args.message);
    send_direct_mesage(&client, &args.keypair, &args.to, ciphertext).unwrap();
    print_messages(&client, &args.keypair);
}

#[derive(StructOpt)]
struct Arguments {
    #[structopt(default_value = "http://localhost:8899")]
    url: String,
    #[structopt(short, long, parse(try_from_str=read_keypair_file), default_value = "~/.chat.keypair")]
    keypair: Keypair,
    #[structopt(short, long)]
    to: Pubkey,
    #[structopt(short, long)]
    message: String,
}

fn send_direct_mesage(
    client: &RpcClient,
    sender: &Keypair,
    receiver: &Pubkey,
    encrypted_text: Vec<u8>,
) -> Result<(), Error> {
    let message_seed: [u8; 8] = rand::thread_rng().gen();
    let (message_pda, _bump) = Pubkey::find_program_address(&[&message_seed], &program::ID);
    let instruction = program::send_direct_mesage(
        sender.pubkey(),
        *receiver,
        message_seed.into(),
        message_pda,
        encrypted_text,
    );
    execute(client, sender, &[instruction], vec![sender])
}

fn execute(
    client: &RpcClient,
    payer: &Keypair,
    instructions: &[Instruction],
    signers: Vec<&Keypair>,
) -> Result<(), Error> {
    let blockhash = client.get_latest_blockhash()?;
    let transaction = Transaction::new_signed_with_payer(
        instructions,
        Some(&payer.pubkey()),
        &signers,
        blockhash,
    );
    client.send_and_confirm_transaction(&transaction)?;
    Ok(())
}

fn print_messages(client: &RpcClient, me: &Keypair) {
    let mailbox_pda = program::mailbox_pda(&me.pubkey());
    let mailbox_account = client.get_account(&mailbox_pda).expect("No messages");
    let mailbox = program::Mailbox::try_deserialize(&mut mailbox_account.data.as_ref())
        .expect("Not a Mailbox account");
    let mut next_message = mailbox.inbox;
    while let Some(message_pda) = next_message {
        let message_account = client
            .get_account(&message_pda)
            .expect("Message PDA not found");
        let message = program::Message::try_deserialize(&mut message_account.data.as_ref())
            .expect("Not a Message account");
        let key = encrypt::SharedKey::new(me, &message.from);
        let text = key.receive_key.decrypt(&message.ciphertext);
        println!("From {}: {}", message.from, text);
        next_message = message.inbox
    }
}
