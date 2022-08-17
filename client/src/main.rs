use rand::Rng;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig, instruction::Instruction, pubkey::Pubkey,
    signature::Keypair, signer::Signer, transaction::Transaction,
};
use structopt::StructOpt;

fn main() {
    let args = Arguments::from_args();
    let client = RpcClient::new_with_commitment(args.url, CommitmentConfig::confirmed());
    // TODO check if exists already
    let chat_address = initialize_direct_chat(&client, &args.keypair, &args.to_address).unwrap();
    // TODO encrypt
    send_direct_mesage(&client, &args.keypair, chat_address, args.message.into()).unwrap();
}

#[derive(StructOpt)]
struct Arguments {
    #[structopt(default_value = "http://localhost:8899")]
    url: String,
    #[structopt(parse(try_from_str=load_keypair), default_value = "~/.chat.keypair")]
    keypair: Keypair,
    to_address: Pubkey,
    message: String,
}

fn load_keypair(path: &str) -> Result<Keypair, std::io::Error> {
    std::fs::read_to_string(path).map(|string| Keypair::from_base58_string(&string))
}

fn initialize_direct_chat(
    client: &RpcClient,
    initiator: &Keypair,
    reciprocator: &Pubkey,
) -> Result<Pubkey, Error> {
    let initiator_pubkey = initiator.pubkey();
    let chat_seed = program::direct_message_seed(&initiator_pubkey, reciprocator);
    let (chat_pda, _chat_bump) = Pubkey::find_program_address(&chat_seed, &program::ID);
    let instruction = program::initialize_direct_chat(initiator.pubkey(), *reciprocator, chat_pda);
    execute(client, initiator, &[instruction], vec![initiator])?;
    Ok(chat_pda)
}

fn send_direct_mesage(
    client: &RpcClient,
    sender: &Keypair,
    chat_address: Pubkey,
    encrypted_text: Vec<u8>,
) -> Result<(), Error> {
    let random_seed = [rand::thread_rng().gen::<u8>()];
    let (message_pda, _bump) = Pubkey::find_program_address(&[&random_seed], &program::ID);
    let instruction =
        program::send_direct_mesage(sender.pubkey(), chat_address, message_pda, encrypted_text);
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
