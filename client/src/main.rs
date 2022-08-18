use anchor_lang::AccountDeserialize;
use encrypt::SharedKey;
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
    let encryption_key = create_encryption_key(&args.keypair, &args.to);
    let ciphertext = encryption_key.transmit_key.encrypt(&args.message);
    // TODO check if exists already
    let chat_address = program::direct_chat_pda(&args.keypair.pubkey(), &args.to);
    send_direct_mesage(&client, &args.keypair, &args.to, &chat_address, ciphertext).unwrap();
    print_messages(&client, &encryption_key, &args.keypair.pubkey(), &args.to);
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
    chat_address: &Pubkey,
    encrypted_text: Vec<u8>,
) -> Result<(), Error> {
    let message_seed: [u8; 8] = rand::thread_rng().gen();
    let (message_pda, _bump) = Pubkey::find_program_address(&[&message_seed], &program::ID);
    let instruction = program::send_direct_mesage(
        sender.pubkey(),
        *receiver,
        *chat_address,
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

fn print_messages(client: &RpcClient, key: &SharedKey, me: &Pubkey, them: &Pubkey) {
    let chat_pda = program::direct_chat_pda(me, them);
    let chat_account = client.get_account(&chat_pda).expect("No messages");
    let chat = program::DirectChat::try_deserialize(&mut chat_account.data.as_ref())
        .expect("Not a DirectChat account");
    let mut next_message = chat.last_message;
    let (junior_key, senior_key) = match me == &chat.junior {
        true => (&key.transmit_key, &key.receive_key),
        false => (&key.receive_key, &key.transmit_key),
    };
    while let Some(message_pda) = next_message {
        let message_account = client
            .get_account(&message_pda)
            .expect("Message PDA not found");
        let message = program::Message::try_deserialize(&mut message_account.data.as_ref())
            .expect("Not a Message account");
        let (indicator, name, decrypt_key) = match message.direction {
            program::Direction::JuniorToSenior => (">>>", chat.junior, junior_key),
            program::Direction::SeniorToJunior => ("<<<", chat.senior, senior_key),
        };
        let text = decrypt_key.decrypt(&message.encrypted_text);
        println!("{} {}: {}", indicator, name, text);
        next_message = message.previous_message
    }
}

fn create_encryption_key(me: &Keypair, them: &Pubkey) -> SharedKey {
    if me.pubkey() < *them {
        SharedKey::new_as_junior(me, them)
    } else {
        SharedKey::new_as_senior(me, them)
    }
}
