use rand::Rng;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig, instruction::Instruction, pubkey::Pubkey,
    signature::read_keypair_file, signature::Keypair, signer::Signer, transaction::Transaction,
};
use structopt::StructOpt;

fn main() {
    let args = Arguments::from_args();
    let client = RpcClient::new_with_commitment(args.url, CommitmentConfig::confirmed());
    // TODO check if exists already
    let chat_address = program::direct_chat_pda(&args.keypair.pubkey(), &args.to);
    send_direct_mesage(&client, &args.keypair, &chat_address, args.message.into()).unwrap();
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
    chat_address: &Pubkey,
    encrypted_text: Vec<u8>,
) -> Result<(), Error> {
    let message_seed: [u8; 8] = rand::thread_rng().gen();
    let (message_pda, _bump) = Pubkey::find_program_address(&[&message_seed], &program::ID);
    let instruction = program::send_direct_mesage(
        sender.pubkey(),
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

type Error = Box<dyn std::error::Error>;
