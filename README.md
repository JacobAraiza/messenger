# Chat Service

Encrypted messages are stored on the chain:
- Immutable record of conversations
- Decentralised storage and searchability

## Direct Messages

- Linked list of messages between two accounts
- Chat PDA with sender and receiver's pubkeys as seed stores head of messages
- Message PDA stores encrypted text, which side it came from, and previous message
