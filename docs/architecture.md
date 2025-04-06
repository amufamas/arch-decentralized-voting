# Arch Network Decentralized Voting Smart Contract - Architecture Design

## Overview
This document outlines the architecture and data structures for an advanced decentralized voting smart contract for the Arch Network using Rust. The contract is designed to be feature-rich, secure, gas-efficient, and user-centric.

## Core Components

### 1. Data Structures

#### Poll Structure
```rust
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct Poll {
    /// Unique identifier for the poll
    pub id: u64,
    /// Creator of the poll
    pub creator: Pubkey,
    /// Title of the poll
    pub title: String,
    /// Description of the poll
    pub description: String,
    /// List of options to vote on
    pub options: Vec<String>,
    /// Start time of the poll (Unix timestamp)
    pub start_time: u64,
    /// End time of the poll (Unix timestamp)
    pub end_time: u64,
    /// Privacy setting (public or private)
    pub is_private: bool,
    /// Whether revoting is allowed
    pub allow_revote: bool,
    /// Whether the poll is active
    pub is_active: bool,
    /// Whether the poll is weighted
    pub is_weighted: bool,
    /// Whether voting is delegated
    pub allow_delegation: bool,
    /// Whether votes are encrypted
    pub is_encrypted: bool,
}
```

#### Vote Structure
```rust
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct Vote {
    /// Poll ID this vote belongs to
    pub poll_id: u64,
    /// Voter's public key
    pub voter: Pubkey,
    /// Selected option index
    pub option_index: u8,
    /// Timestamp of the vote
    pub timestamp: u64,
    /// Weight of the vote (for weighted voting)
    pub weight: u64,
    /// Delegated voter (if applicable)
    pub delegated_to: Option<Pubkey>,
    /// Encrypted vote data (if applicable)
    pub encrypted_data: Option<Vec<u8>>,
    /// Zero-knowledge proof (if applicable)
    pub zk_proof: Option<Vec<u8>>,
}
```

#### VoteCount Structure
```rust
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct VoteCount {
    /// Poll ID this count belongs to
    pub poll_id: u64,
    /// Counts for each option (index corresponds to option index)
    pub counts: Vec<u64>,
    /// Total number of voters
    pub total_voters: u64,
    /// Last updated timestamp
    pub last_updated: u64,
}
```

#### Delegation Structure
```rust
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct Delegation {
    /// Delegator's public key
    pub delegator: Pubkey,
    /// Delegate's public key
    pub delegate: Pubkey,
    /// Poll ID this delegation applies to (None means all polls)
    pub poll_id: Option<u64>,
    /// Expiration time (None means no expiration)
    pub expiration: Option<u64>,
}
```

### 2. Instructions

#### Poll Management Instructions
```rust
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum PollInstruction {
    /// Create a new poll
    /// Accounts:
    /// 0. `[signer, writable]` Poll creator account
    /// 1. `[writable]` New poll account
    /// 2. `[writable]` Vote count account
    CreatePoll {
        title: String,
        description: String,
        options: Vec<String>,
        start_time: u64,
        end_time: u64,
        is_private: bool,
        allow_revote: bool,
        is_weighted: bool,
        allow_delegation: bool,
        is_encrypted: bool,
    },
    
    /// Cancel a poll (only creator can cancel before start time)
    /// Accounts:
    /// 0. `[signer]` Poll creator account
    /// 1. `[writable]` Poll account
    CancelPoll {
        poll_id: u64,
    },
    
    /// Close a poll (automatically done after end time, but can be manually triggered)
    /// Accounts:
    /// 0. `[signer]` Any account (verification happens against poll end time)
    /// 1. `[writable]` Poll account
    ClosePoll {
        poll_id: u64,
    },
}
```

#### Voting Instructions
```rust
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum VoteInstruction {
    /// Cast a vote
    /// Accounts:
    /// 0. `[signer]` Voter account
    /// 1. `[writable]` Vote account
    /// 2. `[writable]` Poll account
    /// 3. `[writable]` Vote count account
    CastVote {
        poll_id: u64,
        option_index: u8,
        weight: Option<u64>,
        encrypted_data: Option<Vec<u8>>,
        zk_proof: Option<Vec<u8>>,
    },
    
    /// Change a vote (if allowed by poll)
    /// Accounts:
    /// 0. `[signer]` Voter account
    /// 1. `[writable]` Vote account
    /// 2. `[writable]` Poll account
    /// 3. `[writable]` Vote count account
    ChangeVote {
        poll_id: u64,
        new_option_index: u8,
        new_encrypted_data: Option<Vec<u8>>,
        new_zk_proof: Option<Vec<u8>>,
    },
    
    /// Delegate voting power
    /// Accounts:
    /// 0. `[signer]` Delegator account
    /// 1. `[writable]` Delegation account
    /// 2. `[]` Delegate account
    DelegateVote {
        poll_id: Option<u64>,
        delegate: Pubkey,
        expiration: Option<u64>,
    },
    
    /// Revoke delegation
    /// Accounts:
    /// 0. `[signer]` Delegator account
    /// 1. `[writable]` Delegation account
    RevokeDelegation {
        delegation_id: u64,
    },
}
```

#### Results Instructions
```rust
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum ResultsInstruction {
    /// Get poll results
    /// Accounts:
    /// 0. `[]` Poll account
    /// 1. `[]` Vote count account
    GetResults {
        poll_id: u64,
    },
    
    /// Decrypt and reveal results (for encrypted polls)
    /// Accounts:
    /// 0. `[signer]` Poll creator account
    /// 1. `[writable]` Poll account
    /// 2. `[writable]` Vote count account
    DecryptResults {
        poll_id: u64,
        decryption_key: Vec<u8>,
    },
}
```

### 3. Error Handling

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VotingError {
    /// Invalid poll parameters
    InvalidPollParameters,
    /// Poll already exists
    PollAlreadyExists,
    /// Poll does not exist
    PollDoesNotExist,
    /// Poll is not active
    PollNotActive,
    /// Poll has not started yet
    PollNotStarted,
    /// Poll has already ended
    PollEnded,
    /// Only the creator can perform this action
    NotPollCreator,
    /// Voter has already voted
    AlreadyVoted,
    /// Revoting is not allowed for this poll
    RevotingNotAllowed,
    /// Invalid option index
    InvalidOptionIndex,
    /// Invalid vote weight
    InvalidVoteWeight,
    /// Invalid delegation
    InvalidDelegation,
    /// Delegation expired
    DelegationExpired,
    /// Invalid zero-knowledge proof
    InvalidZkProof,
    /// Invalid encryption
    InvalidEncryption,
    /// Insufficient fees
    InsufficientFees,
    /// Invalid fee transaction
    InvalidFeeTransaction,
}

impl From<VotingError> for ProgramError {
    fn from(e: VotingError) -> Self {
        ProgramError::Custom(match e {
            VotingError::InvalidPollParameters => 1001,
            VotingError::PollAlreadyExists => 1002,
            VotingError::PollDoesNotExist => 1003,
            VotingError::PollNotActive => 1004,
            VotingError::PollNotStarted => 1005,
            VotingError::PollEnded => 1006,
            VotingError::NotPollCreator => 1007,
            VotingError::AlreadyVoted => 1008,
            VotingError::RevotingNotAllowed => 1009,
            VotingError::InvalidOptionIndex => 1010,
            VotingError::InvalidVoteWeight => 1011,
            VotingError::InvalidDelegation => 1012,
            VotingError::DelegationExpired => 1013,
            VotingError::InvalidZkProof => 1014,
            VotingError::InvalidEncryption => 1015,
            VotingError::InsufficientFees => 1016,
            VotingError::InvalidFeeTransaction => 1017,
        })
    }
}
```

## Architecture Diagram

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

## Gas Optimization Strategies

1. **Compressed Storage**
   - Use bitmaps for tracking voters (one bit per voter)
   - Use efficient data structures (fixed-size arrays where possible)
   - Minimize string lengths with validation

2. **Batch Operations**
   - Allow batch vote processing
   - Batch result calculations

3. **Lazy Evaluation**
   - Calculate results only when requested
   - Update vote counts incrementally

4. **Memory Management**
   - Reuse memory where possible
   - Minimize cloning of large data structures

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

## Implementation Plan

1. Implement core poll creation functionality
2. Implement basic voting mechanism
3. Implement results and poll management
4. Add security features and optimizations
5. Implement bonus features (weighted voting, delegation, encryption)
6. Comprehensive testing
7. Documentation and deployment

This architecture provides a solid foundation for building a feature-rich, secure, and gas-efficient decentralized voting system on the Arch Network.
