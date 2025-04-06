# Arch Network Decentralized Voting Smart Contract - Usage Guide

This guide provides detailed instructions on how to use the Arch Network Decentralized Voting Smart Contract.

## Table of Contents

1. [Setting Up](#setting-up)
2. [Creating a Poll](#creating-a-poll)
3. [Casting a Vote](#casting-a-vote)
4. [Changing a Vote](#changing-a-vote)
5. [Viewing Results](#viewing-results)
6. [Closing a Poll](#closing-a-poll)
7. [Advanced Features](#advanced-features)
   - [Weighted Voting](#weighted-voting)
   - [Delegated Voting](#delegated-voting)
   - [Encrypted Voting](#encrypted-voting)
8. [Error Handling](#error-handling)
9. [Best Practices](#best-practices)

## Setting Up

Before using the voting contract, you need to:

1. Have an Arch Network wallet with sufficient funds for transaction fees
2. Import the contract module in your Rust code:

```rust
use arch_voting_contract::{
    Poll, Vote, VoteCount, VoterRegistry, Delegation, TokenBalance, PollInstruction
};
```

## Creating a Poll

### Basic Poll Creation

To create a basic poll:

```rust
// Create a transaction for fees
let fee_tx = create_fee_transaction();

// Create the poll instruction
let create_poll_instruction = PollInstruction::CreatePoll {
    title: "Community Decision".to_string(),
    description: "Vote on the next community project".to_string(),
    options: vec![
        "Project A".to_string(), 
        "Project B".to_string(), 
        "Project C".to_string()
    ],
    start_time: current_time + 3600, // Start in 1 hour
    end_time: current_time + 86400, // End in 24 hours
    is_private: false,
    allow_revote: true,
    is_weighted: false,
    allow_delegation: false,
    is_encrypted: false,
    weight_token: None,
    early_voter_bonus: 0,
    tx_hex: fee_tx,
};

// Send the instruction
let accounts = [
    creator_account,
    poll_account,
    vote_count_account,
    voter_registry_account,
];

process_instruction(&program_id, &accounts, &create_poll_instruction.try_to_vec()?)?;
```

### Advanced Poll Creation

For a poll with advanced features:

```rust
let create_poll_instruction = PollInstruction::CreatePoll {
    title: "Governance Proposal".to_string(),
    description: "Vote on governance proposal #42".to_string(),
    options: vec!["Approve".to_string(), "Reject".to_string()],
    start_time: current_time + 3600,
    end_time: current_time + 604800, // End in 1 week
    is_private: true, // Anonymous voting
    allow_revote: false,
    is_weighted: true, // Token-weighted voting
    allow_delegation: true, // Allow vote delegation
    is_encrypted: true, // Encrypt votes until poll ends
    weight_token: Some(token_pubkey), // Token for weighting
    early_voter_bonus: 10, // 10% bonus for early voters
    tx_hex: fee_tx,
};
```

## Casting a Vote

### Basic Voting

To cast a simple vote:

```rust
let cast_vote_instruction = PollInstruction::CastVote {
    poll_id: 12345,
    option_index: 0, // Vote for the first option
    weight: None, // Use default weight (1 for non-weighted polls)
    encrypted_data: None,
    zk_proof: None,
    nonce: None,
    tx_hex: fee_tx,
};

let accounts = [
    voter_account,
    vote_account,
    poll_account,
    vote_count_account,
    voter_registry_account,
];

process_instruction(&program_id, &accounts, &cast_vote_instruction.try_to_vec()?)?;
```

### Voting with Custom Weight

For weighted voting with a custom weight:

```rust
let cast_vote_instruction = PollInstruction::CastVote {
    poll_id: 12345,
    option_index: 1,
    weight: Some(50), // Custom weight of 50
    encrypted_data: None,
    zk_proof: None,
    nonce: None,
    tx_hex: fee_tx,
};
```

### Voting with Token Balance

For weighted voting using token balance:

```rust
let accounts = [
    voter_account,
    vote_account,
    poll_account,
    vote_count_account,
    voter_registry_account,
    token_balance_account,
    token_account,
];

let cast_vote_instruction = PollInstruction::CastVote {
    poll_id: 12345,
    option_index: 0,
    weight: None, // Will use token balance
    encrypted_data: None,
    zk_proof: None,
    nonce: None,
    tx_hex: fee_tx,
};
```

## Changing a Vote

If the poll allows revoting:

```rust
let change_vote_instruction = PollInstruction::ChangeVote {
    poll_id: 12345,
    new_option_index: 2, // Change to the third option
    new_encrypted_data: None,
    new_zk_proof: None,
    new_nonce: None,
    tx_hex: fee_tx,
};

let accounts = [
    voter_account,
    vote_account,
    poll_account,
    vote_count_account,
];

process_instruction(&program_id, &accounts, &change_vote_instruction.try_to_vec()?)?;
```

## Viewing Results

To view the current results of a poll:

```rust
let get_results_instruction = PollInstruction::GetResults {
    poll_id: 12345,
};

let accounts = [
    poll_account,
    vote_count_account,
];

process_instruction(&program_id, &accounts, &get_results_instruction.try_to_vec()?)?;
```

The results will be logged to the program output. For encrypted polls, results will only be available after decryption.

## Closing a Poll

A poll automatically closes after its end time, but it can also be manually closed:

```rust
let close_poll_instruction = PollInstruction::ClosePoll {
    poll_id: 12345,
    tx_hex: fee_tx,
};

let accounts = [
    caller_account, // Can be any account if poll has ended, must be creator otherwise
    poll_account,
    vote_count_account,
];

process_instruction(&program_id, &accounts, &close_poll_instruction.try_to_vec()?)?;
```

## Advanced Features

### Weighted Voting

#### Setting Up Token Balance

Before using weighted voting, you need to update your token balance:

```rust
let update_token_balance_instruction = PollInstruction::UpdateTokenBalance {
    token: token_pubkey,
    amount: 100, // 100 tokens
    tx_hex: fee_tx,
};

let accounts = [
    owner_account,
    token_balance_account,
    token_account,
];

process_instruction(&program_id, &accounts, &update_token_balance_instruction.try_to_vec()?)?;
```

#### Early Voter Bonus

When creating a poll with early voter bonus, voters who vote earlier receive a higher weight:

```rust
// With a 10% early voter bonus:
// - A voter with 100 tokens voting at the start gets weight = 110
// - A voter with 100 tokens voting halfway through gets weight = 105
// - A voter with 100 tokens voting at the end gets weight = 100
```

### Delegated Voting

#### Delegating Your Vote

To delegate your vote to another wallet:

```rust
let delegate_vote_instruction = PollInstruction::DelegateVote {
    poll_id: Some(12345), // For a specific poll (None for all polls)
    expiration: Some(current_time + 604800), // Expire in 1 week
    tx_hex: fee_tx,
};

let accounts = [
    delegator_account,
    delegation_account,
    delegate_account,
];

process_instruction(&program_id, &accounts, &delegate_vote_instruction.try_to_vec()?)?;
```

#### Revoking a Delegation

To revoke a delegation:

```rust
let revoke_delegation_instruction = PollInstruction::RevokeDelegation {
    delegation_id: 67890,
    tx_hex: fee_tx,
};

let accounts = [
    delegator_account,
    delegation_account,
];

process_instruction(&program_id, &accounts, &revoke_delegation_instruction.try_to_vec()?)?;
```

#### Voting as a Delegate

When voting as a delegate, include the delegation account:

```rust
let accounts = [
    delegate_account, // The delegate is the signer
    vote_account,
    poll_account,
    vote_count_account,
    voter_registry_account,
    delegation_account, // Include the delegation account
];
```

### Encrypted Voting

#### Casting an Encrypted Vote

For encrypted voting, you need to encrypt your vote off-chain and provide the encrypted data:

```rust
// Off-chain: Encrypt your vote using the poll's public key
let (encrypted_data, nonce) = encrypt_vote(option_index, poll_public_key);

let cast_vote_instruction = PollInstruction::CastVote {
    poll_id: 12345,
    option_index: 0, // This can be any valid option, real vote is encrypted
    weight: None,
    encrypted_data: Some(encrypted_data),
    zk_proof: None,
    nonce: Some(nonce),
    tx_hex: fee_tx,
};
```

#### Decrypting Results

After the poll ends, the creator can decrypt and reveal the results:

```rust
let decrypt_results_instruction = PollInstruction::DecryptResults {
    poll_id: 12345,
    decryption_key: decryption_key, // Private key corresponding to the poll's public key
    tx_hex: fee_tx,
};

let accounts = [
    creator_account,
    poll_account,
    vote_count_account,
    // Include all vote accounts that need to be decrypted
    vote_account_1,
    vote_account_2,
    // ...
];

process_instruction(&program_id, &accounts, &decrypt_results_instruction.try_to_vec()?)?;
```

## Error Handling

The contract returns specific error codes for different failure scenarios:

```rust
// Example error handling
match result {
    Ok(_) => println!("Operation successful"),
    Err(ProgramError::Custom(code)) => {
        match code {
            1001 => println!("Invalid poll parameters"),
            1002 => println!("Poll already exists"),
            1003 => println!("Poll does not exist"),
            1004 => println!("Poll not active"),
            1005 => println!("Poll not started yet"),
            1006 => println!("Poll has already ended"),
            1007 => println!("Only the creator can perform this action"),
            1008 => println!("Already voted"),
            1009 => println!("Revoting not allowed for this poll"),
            // Handle other error codes...
            _ => println!("Unknown error: {}", code),
        }
    },
    Err(err) => println!("Error: {:?}", err),
}
```

## Best Practices

1. **Poll Creation**
   - Keep titles concise (under 100 characters)
   - Provide clear descriptions
   - Limit options to a reasonable number (2-10)
   - Set appropriate time windows

2. **Voting**
   - Vote early for maximum weight in polls with early voter bonus
   - Verify your vote was recorded by checking results

3. **Security**
   - Keep your private keys secure
   - For encrypted polls, don't share your voting choice until after results are revealed
   - Verify delegation expiration times

4. **Gas Efficiency**
   - Batch operations when possible
   - Reuse accounts for multiple operations
   - Close unused accounts to reclaim rent
