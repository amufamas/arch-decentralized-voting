# Arch Network Decentralized Voting Smart Contract

A feature-rich, secure, gas-efficient decentralized voting system for the Arch Network built with Rust.

## Overview

This smart contract provides a comprehensive voting solution that goes beyond basic poll creation and voting. It includes advanced features like anonymous voting with zero-knowledge proofs, weighted voting based on token holdings, vote delegation, and encrypted voting.

## Features

### Core Functionality

- **Poll Creation**: Create customizable polls with title, description, options, start/end times, and privacy settings
- **Voting Mechanism**: Secure one-wallet-one-vote enforcement with signature verification
- **Real-time Results**: Publicly accessible tally function with dynamic display of current poll status
- **Poll Management**: Auto-close polls after end time, creator cancellation, and revote functionality

### Advanced Features

- **Anonymous Voting**: Support for zero-knowledge proofs to enable anonymous voting
- **Weighted Voting**: Vote weight based on token holdings with early voter bonus
- **Delegated Voting**: Allow users to delegate their vote to another wallet
- **Vote Encryption**: Encrypt votes during active poll, decrypt and reveal only after poll ends

### Security & Optimization

- **Gas Optimization**: Compressed storage using bitmaps, efficient data structures, and batch operations
- **Security Measures**: Prevention of reentrancy attacks and double-voting, accurate time checks, and edge case handling

## Architecture

The smart contract is built with a modular architecture that separates concerns and promotes maintainability:

```
+----------------------------------+
|        Voting Smart Contract     |
+----------------------------------+
|                                  |
|  +----------------------------+  |
|  |      Poll Management      |   |
|  |---------------------------|   |
|  | - Create Poll             |   |
|  | - Cancel Poll             |   |
|  | - Close Poll              |   |
|  | - Auto-close mechanism    |   |
|  +----------------------------+  |
|                                  |
|  +----------------------------+  |
|  |      Voting Mechanism     |   |
|  |---------------------------|   |
|  | - Cast Vote               |   |
|  | - Change Vote             |   |
|  | - One wallet = one vote   |   |
|  | - Anonymous voting (ZK)   |   |
|  | - Signature verification  |   |
|  +----------------------------+  |
|                                  |
|  +----------------------------+  |
|  |      Results & Tallying   |   |
|  |---------------------------|   |
|  | - Get Results             |   |
|  | - Real-time tallying      |   |
|  | - Decrypt Results         |   |
|  +----------------------------+  |
|                                  |
|  +----------------------------+  |
|  |      Bonus Features       |   |
|  |---------------------------|   |
|  | - Weighted Voting         |   |
|  | - Delegated Voting        |   |
|  | - Vote Encryption         |   |
|  +----------------------------+  |
|                                  |
+----------------------------------+
```

## Data Structures

### Poll

```rust
pub struct Poll {
    pub id: u64,
    pub creator: Pubkey,
    pub title: String,
    pub description: String,
    pub options: Vec<String>,
    pub start_time: u64,
    pub end_time: u64,
    pub is_private: bool,
    pub allow_revote: bool,
    pub is_active: bool,
    pub is_weighted: bool,
    pub allow_delegation: bool,
    pub is_encrypted: bool,
    pub decryption_key: Option<Vec<u8>>,
    pub weight_token: Option<Pubkey>,
    pub early_voter_bonus: u8,
}
```

### Vote

```rust
pub struct Vote {
    pub poll_id: u64,
    pub voter: Pubkey,
    pub option_index: u8,
    pub timestamp: u64,
    pub weight: u64,
    pub delegated_to: Option<Pubkey>,
    pub encrypted_data: Option<Vec<u8>>,
    pub zk_proof: Option<Vec<u8>>,
    pub nonce: Option<Vec<u8>>,
}
```

### VoteCount

```rust
pub struct VoteCount {
    pub poll_id: u64,
    pub counts: Vec<u64>,
    pub total_voters: u64,
    pub last_updated: u64,
    pub is_finalized: bool,
}
```

### VoterRegistry

```rust
pub struct VoterRegistry {
    pub poll_id: u64,
    pub voter_bitmap: Vec<u8>,
    pub voters: Vec<Pubkey>,
}
```

### Delegation

```rust
pub struct Delegation {
    pub id: u64,
    pub delegator: Pubkey,
    pub delegate: Pubkey,
    pub poll_id: Option<u64>,
    pub expiration: Option<u64>,
    pub is_active: bool,
}
```

### TokenBalance

```rust
pub struct TokenBalance {
    pub owner: Pubkey,
    pub token: Pubkey,
    pub amount: u64,
    pub last_updated: u64,
}
```

## Instructions

The contract supports the following instructions:

1. `CreatePoll`: Create a new poll with customizable options
2. `CancelPoll`: Cancel a poll (only creator can cancel before start time)
3. `CastVote`: Cast a vote for a specific option
4. `ChangeVote`: Change a vote (if allowed by poll)
5. `ClosePoll`: Close a poll (automatically done after end time, but can be manually triggered)
6. `GetResults`: Get poll results
7. `DecryptResults`: Decrypt and reveal results (for encrypted polls)
8. `DelegateVote`: Delegate voting power to another wallet
9. `RevokeDelegation`: Revoke a delegation
10. `UpdateTokenBalance`: Update token balance (for weighted voting)

## Gas Optimization Techniques

1. **Bitmap for voter tracking**
   - Using a bitmap to efficiently track voters
   - Each bit represents a voter, allowing tracking of up to 8192 voters in just 1024 bytes

2. **Efficient data structures**
   - Using fixed-size arrays where possible
   - Using Option<T> only when necessary
   - Minimizing string lengths with validation

3. **Lazy evaluation**
   - Vote counts are only calculated when requested
   - Results are only finalized when needed

4. **Memory management**
   - Reusing accounts where possible
   - Minimizing cloning of large data structures
   - Using references instead of copying data

5. **Batch operations**
   - Supporting batch vote processing
   - Batch result calculations

## Security Considerations

1. **Reentrancy Prevention**
   - Complete state modifications before external calls
   - Use reentrancy guards

2. **Double-Voting Prevention**
   - Track voters in a bitmap
   - Verify signatures
   - Use zero-knowledge proofs for anonymous voting

3. **Time-Based Security**
   - Use secure time sources
   - Validate time ranges
   - Prevent voting before start or after end

4. **Edge Case Handling**
   - Validate all inputs
   - Handle potential overflows
   - Graceful error handling

## Usage Guide

### Creating a Poll

To create a poll, you need to provide:

1. Title and description
2. List of options
3. Start and end times
4. Privacy settings
5. Whether revoting is allowed
6. Whether weighted voting is enabled
7. Whether delegation is allowed
8. Whether votes should be encrypted

Example:

```rust
let create_poll_instruction = PollInstruction::CreatePoll {
    title: "Important Decision".to_string(),
    description: "We need to decide on this important matter".to_string(),
    options: vec!["Option A".to_string(), "Option B".to_string(), "Option C".to_string()],
    start_time: current_time + 3600, // Start in 1 hour
    end_time: current_time + 86400, // End in 24 hours
    is_private: false,
    allow_revote: true,
    is_weighted: false,
    allow_delegation: false,
    is_encrypted: false,
    weight_token: None,
    early_voter_bonus: 0,
    tx_hex: create_fee_transaction(),
};
```

### Casting a Vote

To cast a vote, you need to provide:

1. Poll ID
2. Option index
3. Weight (optional, for weighted voting)
4. Encrypted data (optional, for encrypted voting)
5. Zero-knowledge proof (optional, for private voting)
6. Nonce (optional, for encrypted voting)

Example:

```rust
let cast_vote_instruction = PollInstruction::CastVote {
    poll_id: 12345,
    option_index: 1, // Vote for Option B
    weight: None, // Use default weight
    encrypted_data: None,
    zk_proof: None,
    nonce: None,
    tx_hex: create_fee_transaction(),
};
```

### Delegating a Vote

To delegate your vote to another wallet:

```rust
let delegate_vote_instruction = PollInstruction::DelegateVote {
    poll_id: Some(12345), // For a specific poll (None for all polls)
    expiration: Some(current_time + 604800), // Expire in 1 week
    tx_hex: create_fee_transaction(),
};
```

### Getting Poll Results

To get the results of a poll:

```rust
let get_results_instruction = PollInstruction::GetResults {
    poll_id: 12345,
};
```

## Testing

The contract includes comprehensive unit tests covering:

1. Poll creation
2. Voting
3. Changing votes
4. Closing polls
5. Delegated voting
6. Weighted voting
7. Encrypted voting

Run the tests with:

```
cargo test
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.
