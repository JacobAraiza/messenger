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
    match args.command {
        Command::Send { to, message } => send(&client, &args.keypair, &to, &message).unwrap(),
        Command::GetUnread => {
            for message in all_messages(&client, &args.keypair.pubkey()) {
                println!("From {}: {}", message.from, message.decrypt(&args.keypair));
            }
        }
    }
}

#[derive(StructOpt)]
struct Arguments {
    #[structopt(default_value = "http://localhost:8899")]
    url: String,
    #[structopt(short, long, parse(try_from_str=read_keypair_file), default_value = "~/.chat.keypair")]
    keypair: Keypair,
    #[structopt(subcommand)]
    command: Command,
}

#[derive(StructOpt)]
enum Command {
    Send {
        #[structopt(short, long)]
        to: Pubkey,
        #[structopt(short, long)]
        message: String,
    },
    GetUnread,
}

fn send(client: &RpcClient, from: &Keypair, to: &Pubkey, message: &str) -> Result<(), Error> {
    let key = encrypt::SharedKey::new(from, to);
    let ciphertext = key.transmit_key.encrypt(message);
    let message_seed: [u8; 8] = rand::thread_rng().gen();
    let (message_pda, _bump) = Pubkey::find_program_address(&[&message_seed], &program::ID);
    let instruction = program::send_direct_mesage(
        from.pubkey(),
        *to,
        message_seed.into(),
        message_pda,
        ciphertext,
    );
    execute(client, from, &[instruction], vec![from])
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

#[derive(serde::Serialize, serde::Deserialize)]
struct State {
    read_messages: Option<Pubkey>,
}

fn all_messages<'a, 'b>(
    client: &'a RpcClient,
    to: &'b Pubkey,
) -> impl Iterator<Item = program::Message> + 'a {
    let mailbox_pda = program::mailbox_pda(to);
    let mut next_message = client
        .get_account_with_commitment(&mailbox_pda, CommitmentConfig::confirmed())
        .expect("Could not fetch account")
        .value
        .and_then(|mailbox_account| {
            program::Mailbox::try_deserialize(&mut mailbox_account.data.as_ref())
                .expect("Not a Mailbox account")
                .inbox
        });
    std::iter::from_fn(move || {
        let message_account = client
            .get_account(&next_message?)
            .expect("Message PDA not found");
        let message = program::Message::try_deserialize(&mut message_account.data.as_ref())
            .expect("Not a Message account");
        next_message = message.inbox;
        Some(message)
    })
}

trait Decrypt {
    fn decrypt(&self, to: &Keypair) -> String;
}

impl Decrypt for program::Message {
    fn decrypt(&self, to: &Keypair) -> String {
        let key = encrypt::SharedKey::new(to, &self.from);
        key.receive_key.decrypt(&self.ciphertext)
    }
}
