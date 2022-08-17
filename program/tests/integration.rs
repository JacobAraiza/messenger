use solana_program::instruction::Instruction;
use solana_program_test::{processor, tokio, BanksClientError, ProgramTest, ProgramTestContext};
use solana_sdk::{
    account::AccountSharedData, pubkey::Pubkey, signature::Keypair, signer::Signer,
    transaction::Transaction,
};

#[tokio::test]
async fn test_chat_program() {
    let mut validator = ProgramTest::default();
    validator.add_program("program", program::ID, processor!(program::entry));
    let initiator = add_account(&mut validator);
    println!("Intiator: {}", initiator.pubkey());
    let reciprocator = add_account(&mut validator);
    let mut context = validator.start_with_context().await;
    let chat_pda = initialize_direct_chat(&mut context, &initiator, reciprocator.pubkey())
        .await
        .unwrap();
    println!("Chat PDA Address: {}", chat_pda);
}

fn add_account(validator: &mut ProgramTest) -> Keypair {
    let keypair = Keypair::new();
    let account = AccountSharedData::new(1_000_000_000, 0, &solana_sdk::system_program::id());
    validator.add_account(keypair.pubkey(), account.into());
    keypair
}

async fn initialize_direct_chat(
    context: &mut ProgramTestContext,
    initiator: &Keypair,
    reciprocator: Pubkey,
) -> Result<Pubkey, BanksClientError> {
    let initiator_pubkey = initiator.pubkey();
    let chat_seed = program::direct_message_seed(&initiator_pubkey, &reciprocator);
    let (chat_pda, _chat_bump) = Pubkey::find_program_address(&chat_seed, &program::ID);
    let instruction = program::initialize_direct_chat(initiator.pubkey(), reciprocator, chat_pda);
    execute(context, initiator, &[instruction], vec![initiator]).await?;
    Ok(chat_pda)
}

async fn execute(
    context: &mut ProgramTestContext,
    payer: &Keypair,
    instructions: &[Instruction],
    signers: Vec<&Keypair>,
) -> Result<(), BanksClientError> {
    let transaction = Transaction::new_signed_with_payer(
        instructions,
        Some(&payer.pubkey()),
        &signers,
        context.last_blockhash,
    );
    context.banks_client.process_transaction(transaction).await
}
