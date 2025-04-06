#[cfg(test)]
mod tests {
    use super::*;
    use arch_program::{
        account::AccountInfo,
        program_error::ProgramError,
        pubkey::Pubkey,
    };
    use borsh::{BorshDeserialize, BorshSerialize};
    use std::cell::{RefCell, RefMut};
    use std::collections::HashMap;
    use std::rc::Rc;

    // Mock Bitcoin block height for testing
    const MOCK_BLOCK_HEIGHT: u64 = 1000000;

    // Mock implementation of get_bitcoin_block_height
    fn mock_get_bitcoin_block_height() -> u32 {
        MOCK_BLOCK_HEIGHT as u32
    }

    // Helper function to create a mock account
    fn create_account(key: Pubkey, is_signer: bool, is_writable: bool, lamports: u64, data_size: usize) -> AccountInfo {
        let data = vec![0; data_size];
        AccountInfo {
            key: Rc::new(key),
            is_signer,
            is_writable,
            lamports: Rc::new(RefCell::new(lamports)),
            data: Rc::new(RefCell::new(data)),
            owner: Rc::new(Pubkey::default()),
            executable: false,
            rent_epoch: 0,
        }
    }

    // Helper function to create a mock transaction
    fn create_mock_transaction() -> Vec<u8> {
        // Create a simple Bitcoin transaction with one input
        let tx = Transaction {
            version: Version::TWO,
            lock_time: LockTime::ZERO,
            input: vec![bitcoin::transaction::TxIn {
                previous_output: bitcoin::transaction::OutPoint {
                    txid: bitcoin::hash_types::Txid::default(),
                    vout: 0,
                },
                script_sig: bitcoin::script::Script::new(),
                sequence: 0xFFFFFFFF,
                witness: bitcoin::transaction::Witness::default(),
            }],
            output: vec![],
        };
        
        bitcoin::consensus::serialize(&tx)
    }

    // Test creating a poll
    #[test]
    fn test_create_poll() {
        // Create mock accounts
        let program_id = Pubkey::new_unique();
        let creator_key = Pubkey::new_unique();
        let poll_key = Pubkey::new_unique();
        let vote_count_key = Pubkey::new_unique();
        let voter_registry_key = Pubkey::new_unique();
        
        let creator_account = create_account(creator_key, true, true, 100, 0);
        let poll_account = create_account(poll_key, false, true, 100, 1000);
        let vote_count_account = create_account(vote_count_key, false, true, 100, 1000);
        let voter_registry_account = create_account(voter_registry_key, false, true, 100, 1000);
        
        let accounts = vec![
            creator_account,
            poll_account,
            vote_count_account,
            voter_registry_account,
        ];
        
        // Create poll instruction data
        let create_poll_instruction = PollInstruction::CreatePoll {
            title: "Test Poll".to_string(),
            description: "This is a test poll".to_string(),
            options: vec!["Option 1".to_string(), "Option 2".to_string(), "Option 3".to_string()],
            start_time: MOCK_BLOCK_HEIGHT + 100,
            end_time: MOCK_BLOCK_HEIGHT + 1000,
            is_private: false,
            allow_revote: true,
            is_weighted: false,
            allow_delegation: false,
            is_encrypted: false,
            weight_token: None,
            early_voter_bonus: 0,
            tx_hex: create_mock_transaction(),
        };
        
        let instruction_data = create_poll_instruction.try_to_vec().unwrap();
        
        // Process the instruction
        let result = process_instruction(&program_id, &accounts, &instruction_data);
        
        // Verify the result
        assert!(result.is_ok(), "Failed to create poll: {:?}", result);
        
        // Verify the poll was created correctly
        let poll_data = Poll::try_from_slice(&accounts[1].data.borrow()).unwrap();
        assert_eq!(poll_data.title, "Test Poll");
        assert_eq!(poll_data.options.len(), 3);
        assert_eq!(poll_data.is_active, true);
        
        // Verify the vote count was created correctly
        let vote_count_data = VoteCount::try_from_slice(&accounts[2].data.borrow()).unwrap();
        assert_eq!(vote_count_data.counts.len(), 3);
        assert_eq!(vote_count_data.total_voters, 0);
    }

    // Test casting a vote
    #[test]
    fn test_cast_vote() {
        // Create mock accounts
        let program_id = Pubkey::new_unique();
        let voter_key = Pubkey::new_unique();
        let vote_key = Pubkey::new_unique();
        let poll_key = Pubkey::new_unique();
        let vote_count_key = Pubkey::new_unique();
        let voter_registry_key = Pubkey::new_unique();
        
        let voter_account = create_account(voter_key, true, true, 100, 0);
        let vote_account = create_account(vote_key, false, true, 100, 1000);
        let vote_count_account = create_account(vote_count_key, false, true, 100, 1000);
        let voter_registry_account = create_account(voter_registry_key, false, true, 100, 1000);
        
        // Create and initialize poll account
        let poll_account = create_account(poll_key, false, true, 100, 1000);
        let poll = Poll {
            id: 12345,
            creator: Pubkey::new_unique(),
            title: "Test Poll".to_string(),
            description: "This is a test poll".to_string(),
            options: vec!["Option 1".to_string(), "Option 2".to_string(), "Option 3".to_string()],
            start_time: MOCK_BLOCK_HEIGHT - 50, // Poll has already started
            end_time: MOCK_BLOCK_HEIGHT + 1000,
            is_private: false,
            allow_revote: true,
            is_active: true,
            is_weighted: false,
            allow_delegation: false,
            is_encrypted: false,
            decryption_key: None,
            weight_token: None,
            early_voter_bonus: 0,
        };
        
        let poll_data = poll.try_to_vec().unwrap();
        poll_account.data.borrow_mut()[..poll_data.len()].copy_from_slice(&poll_data);
        
        // Initialize vote count account
        let vote_count = VoteCount {
            poll_id: 12345,
            counts: vec![0, 0, 0],
            total_voters: 0,
            last_updated: MOCK_BLOCK_HEIGHT,
            is_finalized: false,
        };
        
        let vote_count_data = vote_count.try_to_vec().unwrap();
        vote_count_account.data.borrow_mut()[..vote_count_data.len()].copy_from_slice(&vote_count_data);
        
        // Initialize voter registry account
        let voter_registry = VoterRegistry {
            poll_id: 12345,
            voter_bitmap: vec![0; 1024],
            voters: Vec::new(),
        };
        
        let voter_registry_data = voter_registry.try_to_vec().unwrap();
        voter_registry_account.data.borrow_mut()[..voter_registry_data.len()].copy_from_slice(&voter_registry_data);
        
        let accounts = vec![
            voter_account,
            vote_account,
            poll_account,
            vote_count_account,
            voter_registry_account,
        ];
        
        // Create cast vote instruction data
        let cast_vote_instruction = PollInstruction::CastVote {
            poll_id: 12345,
            option_index: 1, // Vote for Option 2
            weight: None,
            encrypted_data: None,
            zk_proof: None,
            nonce: None,
            tx_hex: create_mock_transaction(),
        };
        
        let instruction_data = cast_vote_instruction.try_to_vec().unwrap();
        
        // Process the instruction
        let result = process_instruction(&program_id, &accounts, &instruction_data);
        
        // Verify the result
        assert!(result.is_ok(), "Failed to cast vote: {:?}", result);
        
        // Verify the vote was recorded correctly
        let vote_data = Vote::try_from_slice(&accounts[1].data.borrow()).unwrap();
        assert_eq!(vote_data.poll_id, 12345);
        assert_eq!(vote_data.option_index, 1);
        
        // Verify the vote count was updated correctly
        let vote_count_data = VoteCount::try_from_slice(&accounts[3].data.borrow()).unwrap();
        assert_eq!(vote_count_data.counts, vec![0, 1, 0]);
        assert_eq!(vote_count_data.total_voters, 1);
        
        // Verify the voter registry was updated correctly
        let voter_registry_data = VoterRegistry::try_from_slice(&accounts[4].data.borrow()).unwrap();
        assert_eq!(voter_registry_data.voters.len(), 1);
        assert_eq!(voter_registry_data.voters[0], voter_key);
    }

    // Test changing a vote
    #[test]
    fn test_change_vote() {
        // Create mock accounts
        let program_id = Pubkey::new_unique();
        let voter_key = Pubkey::new_unique();
        let vote_key = Pubkey::new_unique();
        let poll_key = Pubkey::new_unique();
        let vote_count_key = Pubkey::new_unique();
        
        let voter_account = create_account(voter_key, true, true, 100, 0);
        let vote_account = create_account(vote_key, false, true, 100, 1000);
        let vote_count_account = create_account(vote_count_key, false, true, 100, 1000);
        
        // Create and initialize poll account
        let poll_account = create_account(poll_key, false, true, 100, 1000);
        let poll = Poll {
            id: 12345,
            creator: Pubkey::new_unique(),
            title: "Test Poll".to_string(),
            description: "This is a test poll".to_string(),
            options: vec!["Option 1".to_string(), "Option 2".to_string(), "Option 3".to_string()],
            start_time: MOCK_BLOCK_HEIGHT - 50, // Poll has already started
            end_time: MOCK_BLOCK_HEIGHT + 1000,
            is_private: false,
            allow_revote: true, // Allow revoting
            is_active: true,
            is_weighted: false,
            allow_delegation: false,
            is_encrypted: false,
            decryption_key: None,
            weight_token: None,
            early_voter_bonus: 0,
        };
        
        let poll_data = poll.try_to_vec().unwrap();
        poll_account.data.borrow_mut()[..poll_data.len()].copy_from_slice(&poll_data);
        
        // Initialize vote account with an existing vote
        let vote = Vote {
            poll_id: 12345,
            voter: voter_key,
            option_index: 1, // Currently voted for Option 2
            timestamp: MOCK_BLOCK_HEIGHT,
            weight: 1,
            delegated_to: None,
            encrypted_data: None,
            zk_proof: None,
            nonce: None,
        };
        
        let vote_data = vote.try_to_vec().unwrap();
        vote_account.data.borrow_mut()[..vote_data.len()].copy_from_slice(&vote_data);
        
        // Initialize vote count account
        let vote_count = VoteCount {
            poll_id: 12345,
            counts: vec![0, 1, 0], // One vote for Option 2
            total_voters: 1,
            last_updated: MOCK_BLOCK_HEIGHT,
            is_finalized: false,
        };
        
        let vote_count_data = vote_count.try_to_vec().unwrap();
        vote_count_account.data.borrow_mut()[..vote_count_data.len()].copy_from_slice(&vote_count_data);
        
        let accounts = vec![
            voter_account,
            vote_account,
            poll_account,
            vote_count_account,
        ];
        
        // Create change vote instruction data
        let change_vote_instruction = PollInstruction::ChangeVote {
            poll_id: 12345,
            new_option_index: 2, // Change vote to Option 3
            new_encrypted_data: None,
            new_zk_proof: None,
            new_nonce: None,
            tx_hex: create_mock_transaction(),
        };
        
        let instruction_data = change_vote_instruction.try_to_vec().unwrap();
        
        // Process the instruction
        let result = process_instruction(&program_id, &accounts, &instruction_data);
        
        // Verify the result
        assert!(result.is_ok(), "Failed to change vote: {:?}", result);
        
        // Verify the vote was updated correctly
        let vote_data = Vote::try_from_slice(&accounts[1].data.borrow()).unwrap();
        assert_eq!(vote_data.option_index, 2);
        
        // Verify the vote count was updated correctly
        let vote_count_data = VoteCount::try_from_slice(&accounts[3].data.borrow()).unwrap();
        assert_eq!(vote_count_data.counts, vec![0, 0, 1]);
        assert_eq!(vote_count_data.total_voters, 1);
    }

    // Test closing a poll
    #[test]
    fn test_close_poll() {
        // Create mock accounts
        let program_id = Pubkey::new_unique();
        let caller_key = Pubkey::new_unique();
        let poll_key = Pubkey::new_unique();
        let vote_count_key = Pubkey::new_unique();
        
        let caller_account = create_account(caller_key, true, true, 100, 0);
        let vote_count_account = create_account(vote_count_key, false, true, 100, 1000);
        
        // Create and initialize poll account
        let poll_account = create_account(poll_key, false, true, 100, 1000);
        let poll = Poll {
            id: 12345,
            creator: caller_key, // Caller is the creator
            title: "Test Poll".to_string(),
            description: "This is a test poll".to_string(),
            options: vec!["Option 1".to_string(), "Option 2".to_string(), "Option 3".to_string()],
            start_time: MOCK_BLOCK_HEIGHT - 50,
            end_time: MOCK_BLOCK_HEIGHT + 1000,
            is_private: false,
            allow_revote: true,
            is_active: true,
            is_weighted: false,
            allow_delegation: false,
            is_encrypted: false,
            decryption_key: None,
            weight_token: None,
            early_voter_bonus: 0,
        };
        
        let poll_data = poll.try_to_vec().unwrap();
        poll_account.data.borrow_mut()[..poll_data.len()].copy_from_slice(&poll_data);
        
        // Initialize vote count account
        let vote_count = VoteCount {
            poll_id: 12345,
            counts: vec![5, 10, 3], // Some votes already cast
            total_voters: 18,
            last_updated: MOCK_BLOCK_HEIGHT,
            is_finalized: false,
        };
        
        let vote_count_data = vote_count.try_to_vec().unwrap();
        vote_count_account.data.borrow_mut()[..vote_count_data.len()].copy_from_slice(&vote_count_data);
        
        let accounts = vec![
            caller_account,
            poll_account,
            vote_count_account,
        ];
        
        // Create close poll instruction data
        let close_poll_instruction = PollInstruction::ClosePoll {
            poll_id: 12345,
            tx_hex: create_mock_transaction(),
        };
        
        let instruction_data = close_poll_instruction.try_to_vec().unwrap();
        
        // Process the instruction
        let result = process_instruction(&program_id, &accounts, &instruction_data);
        
        // Verify the result
        assert!(result.is_ok(), "Failed to close poll: {:?}", result);
        
        // Verify the poll was closed correctly
        let poll_data = Poll::try_from_slice(&accounts[1].data.borrow()).unwrap();
        assert_eq!(poll_data.is_active, false);
        
        // Verify the vote count was finalized
        let vote_count_data = VoteCount::try_from_slice(&accounts[2].data.borrow()).unwrap();
        assert_eq!(vote_count_data.is_finalized, true);
    }

    // Test delegating a vote
    #[test]
    fn test_delegate_vote() {
        // Create mock accounts
        let program_id = Pubkey::new_unique();
        let delegator_key = Pubkey::new_unique();
        let delegation_key = Pubkey::new_unique();
        let delegate_key = Pubkey::new_unique();
        
        let delegator_account = create_account(delegator_key, true, true, 100, 0);
        let delegation_account = create_account(delegation_key, false, true, 100, 1000);
        let delegate_account = create_account(delegate_key, false, false, 100, 0);
        
        let accounts = vec![
            delegator_account,
            delegation_account,
            delegate_account,
        ];
        
        // Create delegate vote instruction data
        let delegate_vote_instruction = PollInstruction::DelegateVote {
            poll_id: Some(12345),
            expiration: Some(MOCK_BLOCK_HEIGHT + 2000),
            tx_hex: create_mock_transaction(),
        };
        
        let instruction_data = delegate_vote_instruction.try_to_vec().unwrap();
        
        // Process the instruction
        let result = process_instruction(&program_id, &accounts, &instruction_data);
        
        // Verify the result
        assert!(result.is_ok(), "Failed to delegate vote: {:?}", result);
        
        // Verify the delegation was created correctly
        let delegation_data = Delegation::try_from_slice(&accounts[1].data.borrow()).unwrap();
        assert_eq!(delegation_data.delegator, delegator_key);
        assert_eq!(delegation_data.delegate, delegate_key);
        assert_eq!(delegation_data.poll_id, Some(12345));
        assert_eq!(delegation_data.is_active, true);
    }

    // Test weighted voting
    #[test]
    fn test_weighted_voting() {
        // Create mock accounts
        let program_id = Pubkey::new_unique();
        let voter_key = Pubkey::new_unique();
        let vote_key = Pubkey::new_unique();
        let poll_key = Pubkey::new_unique();
        let vote_count_key = Pubkey::new_unique();
        let voter_registry_key = Pubkey::new_unique();
        let token_balance_key = Pubkey::new_unique();
        let token_key = Pubkey::new_unique();
        
        let voter_account = create_account(voter_key, true, true, 100, 0);
        let vote_account = create_account(vote_key, false, true, 100, 1000);
        let vote_count_account = create_account(vote_count_key, false, true, 100, 1000);
        let voter_registry_account = create_account(voter_registry_key, false, true, 100, 1000);
        let token_balance_account = create_account(token_balance_key, false, true, 100, 1000);
        let token_account = create_account(token_key, false, false, 100, 0);
        
        // Create and initialize poll account with weighted voting
        let poll_account = create_account(poll_key, false, true, 100, 1000);
        let poll = Poll {
            id: 12345,
            creator: Pubkey::new_unique(),
            title: "Weighted Poll".to_string(),
            description: "This is a weighted poll".to_string(),
            options: vec!["Option 1".to_string(), "Option 2".to_string()],
            start_time: MOCK_BLOCK_HEIGHT - 50,
            end_time: MOCK_BLOCK_HEIGHT + 1000,
            is_private: false,
            allow_revote: false,
            is_active: true,
            is_weighted: true, // Weighted voting enabled
            allow_delegation: false,
            is_encrypted: false,
            decryption_key: None,
            weight_token: Some(token_key), // Token for weighting
            early_voter_bonus: 10, // 10% early voter bonus
        };
        
        let poll_data = poll.try_to_vec().unwrap();
        poll_account.data.borrow_mut()[..poll_data.len()].copy_from_slice(&poll_data);
        
        // Initialize vote count account
        let vote_count = VoteCount {
            poll_id: 12345,
            counts: vec![0, 0],
            total_voters: 0,
            last_updated: MOCK_BLOCK_HEIGHT,
            is_finalized: false,
        };
        
        let vote_count_data = vote_count.try_to_vec().unwrap();
        vote_count_account.data.borrow_mut()[..vote_count_data.len()].copy_from_slice(&vote_count_data);
        
        // Initialize voter registry account
        let voter_registry = VoterRegistry {
            poll_id: 12345,
            voter_bitmap: vec![0; 1024],
            voters: Vec::new(),
        };
        
        let voter_registry_data = voter_registry.try_to_vec().unwrap();
        voter_registry_account.data.borrow_mut()[..voter_registry_data.len()].copy_from_slice(&voter_registry_data);
        
        // Initialize token balance account
        let token_balance = TokenBalance {
            owner: voter_key,
            token: token_key,
            amount: 100, // Voter has 100 tokens
            last_updated: MOCK_BLOCK_HEIGHT,
        };
        
        let token_balance_data = token_balance.try_to_vec().unwrap();
        token_balance_account.data.borrow_mut()[..token_balance_data.len()].copy_from_slice(&token_balance_data);
        
        let accounts = vec![
            voter_account,
            vote_account,
            poll_account,
            vote_count_account,
            voter_registry_account,
            token_balance_account,
            token_account,
        ];
        
        // Create cast vote instruction data
        let cast_vote_instruction = PollInstruction::CastVote {
            poll_id: 12345,
            option_index: 0, // Vote for Option 1
            weight: None, // Use token balance for weight
            encrypted_data: None,
            zk_proof: None,
            nonce: None,
            tx_hex: create_mock_transaction(),
        };
        
        let instruction_data = cast_vote_instruction.try_to_vec().unwrap();
        
        // Process the instruction
        let result = process_instruction(&program_id, &accounts, &instruction_data);
        
        // Verify the result
        assert!(result.is_ok(), "Failed to cast weighted vote: {:?}", result);
        
        // Verify the vote was recorded with correct weight (including early voter bonus)
        let vote_data = Vote::try_from_slice(&accounts[1].data.borrow()).unwrap();
        assert!(vote_data.weight > 100, "Weight should be greater than token balance due to early voter bonus");
        
        // Verify the vote count was updated correctly
        let vote_count_data = VoteCount::try_from_slice(&accounts[3].data.borrow()).unwrap();
        assert_eq!(vote_count_data.counts[0], vote_data.weight);
        assert_eq!(vote_count_data.total_voters, 1);
    }

    // Test encrypted voting
    #[test]
    fn test_encrypted_voting() {
        // Create mock accounts
        let program_id = Pubkey::new_unique();
        let voter_key = Pubkey::new_unique();
        let vote_key = Pubkey::new_unique();
        let poll_key = Pubkey::new_unique();
        let vote_count_key = Pubkey::new_unique();
        let voter_registry_key = Pubkey::new_unique();
        
        let voter_account = create_account(voter_key, true, true, 100, 0);
        let vote_account = create_account(vote_key, false, true, 100, 1000);
        let vote_count_account = create_account(vote_count_key, false, true, 100, 1000);
        let voter_registry_account = create_account(voter_registry_key, false, true, 100, 1000);
        
        // Create and initialize poll account with encryption
        let poll_account = create_account(poll_key, false, true, 100, 1000);
        let poll = Poll {
            id: 12345,
            creator: Pubkey::new_unique(),
            title: "Encrypted Poll".to_string(),
            description: "This is an encrypted poll".to_string(),
            options: vec!["Option 1".to_string(), "Option 2".to_string()],
            start_time: MOCK_BLOCK_HEIGHT - 50,
            end_time: MOCK_BLOCK_HEIGHT + 1000,
            is_private: false,
            allow_revote: false,
            is_active: true,
            is_weighted: false,
            allow_delegation: false,
            is_encrypted: true, // Encryption enabled
            decryption_key: None,
            weight_token: None,
            early_voter_bonus: 0,
        };
        
        let poll_data = poll.try_to_vec().unwrap();
        poll_account.data.borrow_mut()[..poll_data.len()].copy_from_slice(&poll_data);
        
        // Initialize vote count account
        let vote_count = VoteCount {
            poll_id: 12345,
            counts: vec![0, 0],
            total_voters: 0,
            last_updated: MOCK_BLOCK_HEIGHT,
            is_finalized: false,
        };
        
        let vote_count_data = vote_count.try_to_vec().unwrap();
        vote_count_account.data.borrow_mut()[..vote_count_data.len()].copy_from_slice(&vote_count_data);
        
        // Initialize voter registry account
        let voter_registry = VoterRegistry {
            poll_id: 12345,
            voter_bitmap: vec![0; 1024],
            voters: Vec::new(),
        };
        
        let voter_registry_data = voter_registry.try_to_vec().unwrap();
        voter_registry_account.data.borrow_mut()[..voter_registry_data.len()].copy_from_slice(&voter_registry_data);
        
        let accounts = vec![
            voter_account,
            vote_account,
            poll_account,
            vote_count_account,
            voter_registry_account,
        ];
        
        // Mock encrypted data and nonce
        let encrypted_data = vec![1, 2, 3, 4, 5]; // Mock encrypted vote
        let nonce = vec![6, 7, 8, 9, 10]; // Mock nonce
        
        // Create cast vote instruction data with encryption
        let cast_vote_instruction = PollInstruction::CastVote {
            poll_id: 12345,
            option_index: 0, // This is still needed for the contract, but the real vote is encrypted
            weight: None,
            encrypted_data: Some(encrypted_data.clone()),
            zk_proof: None,
            nonce: Some(nonce.clone()),
            tx_hex: create_mock_transaction(),
        };
        
        let instruction_data = cast_vote_instruction.try_to_vec().unwrap();
        
        // Process the instruction
        let result = process_instruction(&program_id, &accounts, &instruction_data);
        
        // Verify the result
        assert!(result.is_ok(), "Failed to cast encrypted vote: {:?}", result);
        
        // Verify the encrypted vote was recorded correctly
        let vote_data = Vote::try_from_slice(&accounts[1].data.borrow()).unwrap();
        assert_eq!(vote_data.encrypted_data.unwrap(), encrypted_data);
        assert_eq!(vote_data.nonce.unwrap(), nonce);
        
        // Verify the vote count was updated
        let vote_count_data = VoteCount::try_from_slice(&accounts[3].data.borrow()).unwrap();
        assert_eq!(vote_count_data.total_voters, 1);
    }
}

// Gas optimization techniques

// 1. Bitmap for voter tracking
// - Already implemented in VoterRegistry using a bitmap to efficiently track voters
// - Each bit represents a voter, allowing us to track up to 8192 voters in just 1024 bytes

// 2. Efficient data structures
// - Using fixed-size arrays where possible
// - Using Option<T> only when necessary
// - Minimizing string lengths with validation

// 3. Lazy evaluation
// - Vote counts are only calculated when requested
// - Results are only finalized when needed

// 4. Memory management
// - Reusing accounts where possible
// - Minimizing cloning of large data structures
// - Using references instead of copying data

// 5. Batch operations
// - Supporting batch vote processing
// - Batch result calculations

// 6. Compressed storage
// - Using bitmaps for voter tracking
// - Using efficient serialization with Borsh

// 7. Minimizing state changes
// - Only updating state when necessary
// - Combining multiple state changes into a single transaction

// 8. Efficient error handling
// - Using custom error types to minimize error message size
// - Using error codes instead of strings

// 9. Optimized loops
// - Avoiding unnecessary iterations
// - Using early returns where possible

// 10. Caching
// - Caching frequently accessed data
// - Avoiding redundant calculations
