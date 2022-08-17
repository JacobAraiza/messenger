use anchor_lang::AccountDeserialize;
use rand::Rng;
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
    let reciprocator = add_account(&mut validator);
    let mut context = validator.start_with_context().await;
    println!("Intiator: {}", initiator.pubkey());
    println!("Reciprocator: {}", reciprocator.pubkey());

    // Start chat
    let chat_pda = initialize_direct_chat(&mut context, &initiator, reciprocator.pubkey())
        .await
        .unwrap();
    println!("Chat PDA Address: {}", chat_pda);

    // Send first message
    let encrypted_text: Vec<u8> = "Hello".into();
    let first_message_pda =
        send_direct_message(&mut context, &initiator, encrypted_text.clone(), chat_pda)
            .await
            .unwrap();
    println!("First message PDA Address: {}", first_message_pda);

    {
        let message = context
            .banks_client
            .get_account(first_message_pda)
            .await
            .unwrap()
            .unwrap();
        let message_data = program::Message::try_deserialize(&mut message.data.as_ref()).unwrap();
        assert_eq!(
            message_data.direction,
            program::Direction::InitiatorToReciprocator
        );
        assert_eq!(message_data.previous_message, None);
        assert_eq!(message_data.encrypted_text, encrypted_text);

        let chat = context
            .banks_client
            .get_account(chat_pda)
            .await
            .unwrap()
            .unwrap();
        let chat_data = program::DirectChat::try_deserialize(&mut chat.data.as_ref()).unwrap();
        assert_eq!(chat_data.initiator, initiator.pubkey());
        assert_eq!(chat_data.reciprocator, reciprocator.pubkey());
        assert_eq!(chat_data.last_message, Some(first_message_pda));
    }

    // Send second message
    let encrypted_response: Vec<u8> = "Hi! Who's this?".into();
    let second_message_pda = send_direct_message(
        &mut context,
        &reciprocator,
        encrypted_response.clone(),
        chat_pda,
    )
    .await
    .unwrap();
    println!("Second message PDA Address: {}", second_message_pda);

    {
        let message = context
            .banks_client
            .get_account(second_message_pda)
            .await
            .unwrap()
            .unwrap();
        let message_data = program::Message::try_deserialize(&mut message.data.as_ref()).unwrap();
        assert_eq!(
            message_data.direction,
            program::Direction::ReciprocatorToInitiator
        );
        assert_eq!(message_data.previous_message, Some(first_message_pda));
        assert_eq!(message_data.encrypted_text, encrypted_response);

        let chat = context
            .banks_client
            .get_account(chat_pda)
            .await
            .unwrap()
            .unwrap();
        let chat_data = program::DirectChat::try_deserialize(&mut chat.data.as_ref()).unwrap();
        assert_eq!(chat_data.initiator, initiator.pubkey());
        assert_eq!(chat_data.reciprocator, reciprocator.pubkey());
        assert_eq!(chat_data.last_message, Some(second_message_pda));
    }
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

async fn send_direct_message(
    context: &mut ProgramTestContext,
    sender: &Keypair,
    encrypted_text: Vec<u8>,
    chat_pda: Pubkey,
) -> Result<Pubkey, BanksClientError> {
    let from_pubkey = sender.pubkey();
    let seed: [u8; 8] = rand::thread_rng().gen();
    let (message_pda, _bump) = Pubkey::find_program_address(&[&seed], &program::ID);
    let instruction = program::send_direct_mesage(
        from_pubkey,
        chat_pda,
        seed.into(),
        message_pda,
        encrypted_text,
    );
    execute(context, sender, &[instruction], vec![sender]).await?;
    Ok(message_pda)
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
        context.banks_client.get_latest_blockhash().await?,
    );
    context.banks_client.process_transaction(transaction).await
}
