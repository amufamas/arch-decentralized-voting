use arch_program::{
    account::AccountInfo,
    bitcoin::{self, absolute::LockTime, transaction::Version, Transaction},
    entrypoint,
    helper::add_state_transition,
    input_to_sign::InputToSign,
    msg,
    program::{
        get_account_script_pubkey, get_bitcoin_block_height, next_account_info,
        set_transaction_to_sign,
    },
    program_error::ProgramError,
    pubkey::Pubkey,
    transaction_to_sign::TransactionToSign,
};
use borsh::{BorshDeserialize, BorshSerialize};

// Data Structures

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
    /// Decryption key (only set after poll ends if encrypted)
    pub decryption_key: Option<Vec<u8>>,
    /// Token address for weighted voting (if applicable)
    pub weight_token: Option<Pubkey>,
    /// Early voter bonus multiplier (0 means no bonus)
    pub early_voter_bonus: u8,
}

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
    /// Nonce for encryption (if applicable)
    pub nonce: Option<Vec<u8>>,
}

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
    /// Whether results are finalized
    pub is_finalized: bool,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct VoterRegistry {
    /// Poll ID this registry belongs to
    pub poll_id: u64,
    /// Bitmap of voters (for efficient lookup)
    pub voter_bitmap: Vec<u8>,
    /// List of voter public keys (for anonymous voting verification)
    pub voters: Vec<Pubkey>,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct Delegation {
    /// Unique identifier for the delegation
    pub id: u64,
    /// Delegator's public key
    pub delegator: Pubkey,
    /// Delegate's public key
    pub delegate: Pubkey,
    /// Poll ID this delegation applies to (None means all polls)
    pub poll_id: Option<u64>,
    /// Expiration time (None means no expiration)
    pub expiration: Option<u64>,
    /// Whether the delegation is active
    pub is_active: bool,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct TokenBalance {
    /// Owner's public key
    pub owner: Pubkey,
    /// Token address
    pub token: Pubkey,
    /// Balance amount
    pub amount: u64,
    /// Last updated timestamp
    pub last_updated: u64,
}

// Instructions

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum PollInstruction {
    /// Create a new poll
    /// Accounts:
    /// 0. `[signer, writable]` Poll creator account
    /// 1. `[writable]` New poll account
    /// 2. `[writable]` Vote count account
    /// 3. `[writable]` Voter registry account
    /// 4. `[]` Token account (optional, only if weighted voting)
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
        weight_token: Option<Pubkey>,
        early_voter_bonus: u8,
        tx_hex: Vec<u8>, // Raw Bitcoin transaction for fees
    },
    
    /// Cancel a poll (only creator can cancel before start time)
    /// Accounts:
    /// 0. `[signer]` Poll creator account
    /// 1. `[writable]` Poll account
    CancelPoll {
        poll_id: u64,
        tx_hex: Vec<u8>, // Raw Bitcoin transaction for fees
    },
    
    /// Cast a vote
    /// Accounts:
    /// 0. `[signer]` Voter account
    /// 1. `[writable]` Vote account
    /// 2. `[writable]` Poll account
    /// 3. `[writable]` Vote count account
    /// 4. `[writable]` Voter registry account
    /// 5. `[]` Delegation account (optional, only if voting as delegate)
    /// 6. `[]` Token balance account (optional, only if weighted voting)
    CastVote {
        poll_id: u64,
        option_index: u8,
        weight: Option<u64>,
        encrypted_data: Option<Vec<u8>>,
        zk_proof: Option<Vec<u8>>,
        nonce: Option<Vec<u8>>,
        tx_hex: Vec<u8>, // Raw Bitcoin transaction for fees
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
        new_nonce: Option<Vec<u8>>,
        tx_hex: Vec<u8>, // Raw Bitcoin transaction for fees
    },
    
    /// Close a poll (automatically done after end time, but can be manually triggered)
    /// Accounts:
    /// 0. `[signer]` Any account (verification happens against poll end time)
    /// 1. `[writable]` Poll account
    /// 2. `[writable]` Vote count account
    ClosePoll {
        poll_id: u64,
        tx_hex: Vec<u8>, // Raw Bitcoin transaction for fees
    },
    
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
    /// 3. `[writable]` Vote accounts (multiple)
    DecryptResults {
        poll_id: u64,
        decryption_key: Vec<u8>,
        tx_hex: Vec<u8>, // Raw Bitcoin transaction for fees
    },
    
    /// Delegate voting power
    /// Accounts:
    /// 0. `[signer]` Delegator account
    /// 1. `[writable]` Delegation account
    /// 2. `[]` Delegate account
    DelegateVote {
        poll_id: Option<u64>,
        expiration: Option<u64>,
        tx_hex: Vec<u8>, // Raw Bitcoin transaction for fees
    },
    
    /// Revoke delegation
    /// Accounts:
    /// 0. `[signer]` Delegator account
    /// 1. `[writable]` Delegation account
    RevokeDelegation {
        delegation_id: u64,
        tx_hex: Vec<u8>, // Raw Bitcoin transaction for fees
    },
    
    /// Update token balance (for weighted voting)
    /// Accounts:
    /// 0. `[signer]` Owner account
    /// 1. `[writable]` Token balance account
    /// 2. `[]` Token account
    UpdateTokenBalance {
        token: Pubkey,
        amount: u64,
        tx_hex: Vec<u8>, // Raw Bitcoin transaction for fees
    },
}

// Error Handling

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
    /// Poll already started
    PollAlreadyStarted,
    /// Poll not encrypted
    PollNotEncrypted,
    /// Results already finalized
    ResultsAlreadyFinalized,
    /// Invalid decryption key
    InvalidDecryptionKey,
    /// Poll still active
    PollStillActive,
    /// Delegation not found
    DelegationNotFound,
    /// Not delegator
    NotDelegator,
    /// Token balance not found
    TokenBalanceNotFound,
    /// Invalid token
    InvalidToken,
    /// Missing nonce
    MissingNonce,
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
            VotingError::PollAlreadyStarted => 1018,
            VotingError::PollNotEncrypted => 1019,
            VotingError::ResultsAlreadyFinalized => 1020,
            VotingError::InvalidDecryptionKey => 1021,
            VotingError::PollStillActive => 1022,
            VotingError::DelegationNotFound => 1023,
            VotingError::NotDelegator => 1024,
            VotingError::TokenBalanceNotFound => 1025,
            VotingError::InvalidToken => 1026,
            VotingError::MissingNonce => 1027,
        })
    }
}

// Program Entry Point

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    // Deserialize instruction data
    let instruction = PollInstruction::try_from_slice(instruction_data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    // Process the instruction
    match instruction {
        PollInstruction::CreatePoll {
            title,
            description,
            options,
            start_time,
            end_time,
            is_private,
            allow_revote,
            is_weighted,
            allow_delegation,
            is_encrypted,
            weight_token,
            early_voter_bonus,
            tx_hex,
        } => {
            process_create_poll(
                program_id,
                accounts,
                title,
                description,
                options,
                start_time,
                end_time,
                is_private,
                allow_revote,
                is_weighted,
                allow_delegation,
                is_encrypted,
                weight_token,
                early_voter_bonus,
                tx_hex,
            )
        }
        PollInstruction::CancelPoll { poll_id, tx_hex } => {
            process_cancel_poll(program_id, accounts, poll_id, tx_hex)
        }
        PollInstruction::CastVote {
            poll_id,
            option_index,
            weight,
            encrypted_data,
            zk_proof,
            nonce,
            tx_hex,
        } => {
            process_cast_vote(
                program_id,
                accounts,
                poll_id,
                option_index,
                weight,
                encrypted_data,
                zk_proof,
                nonce,
                tx_hex,
            )
        }
        PollInstruction::ChangeVote {
            poll_id,
            new_option_index,
            new_encrypted_data,
            new_zk_proof,
            new_nonce,
            tx_hex,
        } => {
            process_change_vote(
                program_id,
                accounts,
                poll_id,
                new_option_index,
                new_encrypted_data,
                new_zk_proof,
                new_nonce,
                tx_hex,
            )
        }
        PollInstruction::ClosePoll { poll_id, tx_hex } => {
            process_close_poll(program_id, accounts, poll_id, tx_hex)
        }
        PollInstruction::GetResults { poll_id } => {
            process_get_results(program_id, accounts, poll_id)
        }
        PollInstruction::DecryptResults {
            poll_id,
            decryption_key,
            tx_hex,
        } => {
            process_decrypt_results(program_id, accounts, poll_id, decryption_key, tx_hex)
        }
        PollInstruction::DelegateVote {
            poll_id,
            expiration,
            tx_hex,
        } => {
            process_delegate_vote(program_id, accounts, poll_id, expiration, tx_hex)
        }
        PollInstruction::RevokeDelegation {
            delegation_id,
            tx_hex,
        } => {
            process_revoke_delegation(program_id, accounts, delegation_id, tx_hex)
        }
        PollInstruction::UpdateTokenBalance {
            token,
            amount,
            tx_hex,
        } => {
            process_update_token_balance(program_id, accounts, token, amount, tx_hex)
        }
    }
}

// Instruction Processing Functions

fn process_create_poll(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
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
    weight_token: Option<Pubkey>,
    early_voter_bonus: u8,
    tx_hex: Vec<u8>,
) -> Result<(), ProgramError> {
    // Validate accounts
    let min_accounts = if is_weighted && weight_token.is_some() { 5 } else { 4 };
    if accounts.len() < min_accounts {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    let account_iter = &mut accounts.iter();
    
    // Get accounts
    let creator_account = next_account_info(account_iter)?;
    let poll_account = next_account_info(account_iter)?;
    let vote_count_account = next_account_info(account_iter)?;
    let voter_registry_account = next_account_info(account_iter)?;
    
    // Optional token account for weighted voting
    let token_account = if is_weighted && weight_token.is_some() {
        Some(next_account_info(account_iter)?)
    } else {
        None
    };

    // Verify account permissions
    if !creator_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    
    if !poll_account.is_writable || !vote_count_account.is_writable || !voter_registry_account.is_writable {
        return Err(ProgramError::InvalidAccountData);
    }

    // Validate poll parameters
    if title.is_empty() || title.len() > 100 {
        return Err(VotingError::InvalidPollParameters.into());
    }

    if description.len() > 1000 {
        return Err(VotingError::InvalidPollParameters.into());
    }

    if options.is_empty() || options.len() > 20 {
        return Err(VotingError::InvalidPollParameters.into());
    }

    for option in &options {
        if option.is_empty() || option.len() > 100 {
            return Err(VotingError::InvalidPollParameters.into());
        }
    }

    // Validate time parameters
    let current_time = get_bitcoin_block_height() as u64;
    
    if start_time >= end_time {
        return Err(VotingError::InvalidPollParameters.into());
    }

    if end_time <= current_time {
        return Err(VotingError::InvalidPollParameters.into());
    }

    // Validate weighted voting parameters
    if is_weighted && weight_token.is_none() {
        return Err(VotingError::InvalidPollParameters.into());
    }

    if early_voter_bonus > 100 {
        return Err(VotingError::InvalidPollParameters.into());
    }

    // Generate a unique poll ID (using current time and creator's pubkey)
    let poll_id = current_time.wrapping_add(creator_account.key.to_bytes()[0] as u64);

    // Create the poll
    let poll = Poll {
        id: poll_id,
        creator: *creator_account.key,
        title,
        description,
        options: options.clone(),
        start_time,
        end_time,
        is_private,
        allow_revote,
        is_active: true,
        is_weighted,
        allow_delegation,
        is_encrypted,
        decryption_key: None,
        weight_token,
        early_voter_bonus,
    };

    // Create vote count
    let vote_count = VoteCount {
        poll_id,
        counts: vec![0; options.len()],
        total_voters: 0,
        last_updated: current_time,
        is_finalized: false,
    };

    // Create voter registry
    let voter_registry = VoterRegistry {
        poll_id,
        voter_bitmap: vec![0; 1024], // Space for up to 8192 voters (1024 bytes * 8 bits)
        voters: Vec::new(),
    };

    // Serialize and save poll data
    let poll_data = poll.try_to_vec().map_err(|_| ProgramError::InvalidAccountData)?;
    
    // Ensure account has enough space
    let poll_data_len = poll_account.data.borrow().len();
    if poll_data.len() > poll_data_len {
        poll_account.realloc(poll_data.len(), false)?;
    }
    
    poll_account.data.borrow_mut()[..poll_data.len()].copy_from_slice(&poll_data);

    // Serialize and save vote count data
    let vote_count_data = vote_count.try_to_vec().map_err(|_| ProgramError::InvalidAccountData)?;
    
    // Ensure account has enough space
    let vote_count_data_len = vote_count_account.data.borrow().len();
    if vote_count_data.len() > vote_count_data_len {
        vote_count_account.realloc(vote_count_data.len(), false)?;
    }
    
    vote_count_account.data.borrow_mut()[..vote_count_data.len()].copy_from_slice(&vote_count_data);

    // Serialize and save voter registry data
    let voter_registry_data = voter_registry.try_to_vec().map_err(|_| ProgramError::InvalidAccountData)?;
    
    // Ensure account has enough space
    let voter_registry_data_len = voter_registry_account.data.borrow().len();
    if voter_registry_data.len() > voter_registry_data_len {
        voter_registry_account.realloc(voter_registry_data.len(), false)?;
    }
    
    voter_registry_account.data.borrow_mut()[..voter_registry_data.len()].copy_from_slice(&voter_registry_data);

    // Process fee transaction
    process_fee_transaction(accounts, tx_hex)?;

    // Log success
    msg!("Poll created successfully with ID: {}", poll_id);
    
    Ok(())
}

fn process_cancel_poll(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    poll_id: u64,
    tx_hex: Vec<u8>,
) -> Result<(), ProgramError> {
    // Validate accounts
    if accounts.len() < 2 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    let account_iter = &mut accounts.iter();
    
    // Get accounts
    let creator_account = next_account_info(account_iter)?;
    let poll_account = next_account_info(account_iter)?;

    // Verify account permissions
    if !creator_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    
    if !poll_account.is_writable {
        return Err(ProgramError::InvalidAccountData);
    }

    // Deserialize poll data
    let mut poll = Poll::try_from_slice(&poll_account.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    // Verify poll exists and creator is authorized
    if poll.id != poll_id {
        return Err(VotingError::PollDoesNotExist.into());
    }

    if poll.creator != *creator_account.key {
        return Err(VotingError::NotPollCreator.into());
    }

    // Verify poll hasn't started yet
    let current_time = get_bitcoin_block_height() as u64;
    if current_time >= poll.start_time {
        return Err(VotingError::PollAlreadyStarted.into());
    }

    // Update poll status
    poll.is_active = false;

    // Serialize and save updated poll data
    let poll_data = poll.try_to_vec().map_err(|_| ProgramError::InvalidAccountData)?;
    poll_account.data.borrow_mut()[..poll_data.len()].copy_from_slice(&poll_data);

    // Process fee transaction
    process_fee_transaction(accounts, tx_hex)?;

    // Log success
    msg!("Poll cancelled successfully: {}", poll_id);
    
    Ok(())
}

fn process_cast_vote(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    poll_id: u64,
    option_index: u8,
    weight: Option<u64>,
    encrypted_data: Option<Vec<u8>>,
    zk_proof: Option<Vec<u8>>,
    nonce: Option<Vec<u8>>,
    tx_hex: Vec<u8>,
) -> Result<(), ProgramError> {
    // Validate accounts
    if accounts.len() < 5 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    let account_iter = &mut accounts.iter();
    
    // Get accounts
    let voter_account = next_account_info(account_iter)?;
    let vote_account = next_account_info(account_iter)?;
    let poll_account = next_account_info(account_iter)?;
    let vote_count_account = next_account_info(account_iter)?;
    let voter_registry_account = next_account_info(account_iter)?;
    
    // Optional accounts
    let delegation_account = if account_iter.len() > 0 {
        Some(next_account_info(account_iter)?)
    } else {
        None
    };
    
    let token_balance_account = if account_iter.len() > 0 {
        Some(next_account_info(account_iter)?)
    } else {
        None
    };

    // Verify account permissions
    if !voter_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    
    if !vote_account.is_writable || !poll_account.is_writable || 
       !vote_count_account.is_writable || !voter_registry_account.is_writable {
        return Err(ProgramError::InvalidAccountData);
    }

    // Deserialize poll data
    let poll = Poll::try_from_slice(&poll_account.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    // Verify poll exists and is active
    if poll.id != poll_id {
        return Err(VotingError::PollDoesNotExist.into());
    }

    if !poll.is_active {
        return Err(VotingError::PollNotActive.into());
    }

    // Verify poll timing
    let current_time = get_bitcoin_block_height() as u64;
    
    if current_time < poll.start_time {
        return Err(VotingError::PollNotStarted.into());
    }
    
    if current_time > poll.end_time {
        return Err(VotingError::PollEnded.into());
    }

    // Verify option index is valid
    if option_index as usize >= poll.options.len() {
        return Err(VotingError::InvalidOptionIndex.into());
    }

    // Deserialize voter registry
    let mut voter_registry = VoterRegistry::try_from_slice(&voter_registry_account.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    // Check if voter has already voted (one wallet = one vote enforcement)
    let voter_key = voter_account.key;
    let voter_index = find_voter_index(&voter_registry, voter_key);
    
    if let Some(index) = voter_index {
        // Voter has already voted
        if !poll.allow_revote {
            return Err(VotingError::AlreadyVoted.into());
        }
        
        // If revoting is allowed, we'll update the existing vote
        // This will be handled in the vote count update below
    } else {
        // Add voter to registry
        add_voter_to_registry(&mut voter_registry, voter_key)?;
    }

    // Handle delegation if applicable
    let effective_voter = if let Some(delegation_acc) = delegation_account {
        if !poll.allow_delegation {
            return Err(VotingError::InvalidDelegation.into());
        }
        
        let delegation = Delegation::try_from_slice(&delegation_acc.data.borrow())
            .map_err(|_| ProgramError::InvalidAccountData)?;
        
        // Verify delegation is valid for this poll and voter
        if delegation.delegator != *voter_key {
            return Err(VotingError::InvalidDelegation.into());
        }
        
        if !delegation.is_active {
            return Err(VotingError::InvalidDelegation.into());
        }
        
        if let Some(delegation_poll_id) = delegation.poll_id {
            if delegation_poll_id != poll_id {
                return Err(VotingError::InvalidDelegation.into());
            }
        }
        
        // Check if delegation has expired
        if let Some(expiration) = delegation.expiration {
            if current_time > expiration {
                return Err(VotingError::DelegationExpired.into());
            }
        }
        
        // Use delegate as effective voter
        delegation.delegate
    } else {
        *voter_key
    };

    // Verify zero-knowledge proof if poll is private
    if poll.is_private {
        if let Some(proof) = &zk_proof {
            // In a real implementation, we would verify the ZK proof here
            // For simplicity, we'll just check that a proof was provided
            if proof.is_empty() {
                return Err(VotingError::InvalidZkProof.into());
            }
        } else {
            return Err(VotingError::InvalidZkProof.into());
        }
    }

    // Verify encryption if poll is encrypted
    if poll.is_encrypted {
        if encrypted_data.is_none() {
            return Err(VotingError::InvalidEncryption.into());
        }
        
        if nonce.is_none() {
            return Err(VotingError::MissingNonce.into());
        }
    }

    // Determine vote weight
    let vote_weight = if poll.is_weighted {
        if let Some(token_balance_acc) = token_balance_account {
            // Get token balance for weighted voting
            let token_balance = TokenBalance::try_from_slice(&token_balance_acc.data.borrow())
                .map_err(|_| VotingError::TokenBalanceNotFound)?;
            
            // Verify token balance belongs to voter and is for the correct token
            if token_balance.owner != *voter_key {
                return Err(ProgramError::InvalidAccountData);
            }
            
            if let Some(weight_token) = poll.weight_token {
                if token_balance.token != weight_token {
                    return Err(VotingError::InvalidToken.into());
                }
            }
            
            // Apply early voter bonus if applicable
            let time_elapsed = current_time.saturating_sub(poll.start_time);
            let poll_duration = poll.end_time.saturating_sub(poll.start_time);
            
            if poll.early_voter_bonus > 0 && poll_duration > 0 {
                let progress = (time_elapsed as f64) / (poll_duration as f64);
                let bonus_multiplier = 1.0 + (poll.early_voter_bonus as f64 / 100.0) * (1.0 - progress);
                
                // Apply bonus (with ceiling to ensure early voters get at least some bonus)
                let bonus_weight = (token_balance.amount as f64 * bonus_multiplier).ceil() as u64;
                bonus_weight
            } else {
                token_balance.amount
            }
        } else {
            // Use provided weight or default to 1
            weight.unwrap_or(1)
        }
    } else {
        // For non-weighted polls, all votes have weight 1
        1
    };

    // Create or update vote
    let vote = Vote {
        poll_id,
        voter: *voter_key,
        option_index,
        timestamp: current_time,
        weight: vote_weight,
        delegated_to: if delegation_account.is_some() { Some(effective_voter) } else { None },
        encrypted_data,
        zk_proof,
        nonce,
    };

    // Serialize and save vote data
    let vote_data = vote.try_to_vec().map_err(|_| ProgramError::InvalidAccountData)?;
    
    // Ensure account has enough space
    let vote_data_len = vote_account.data.borrow().len();
    if vote_data.len() > vote_data_len {
        vote_account.realloc(vote_data.len(), false)?;
    }
    
    vote_account.data.borrow_mut()[..vote_data.len()].copy_from_slice(&vote_data);

    // Update vote count
    update_vote_count(
        vote_count_account,
        option_index,
        vote_weight,
        voter_index.is_some(), // is_revote
        voter_index.map(|i| get_previous_vote(vote_account, poll_id, voter_key).unwrap_or(0) as u8),
    )?;

    // Update voter registry
    let voter_registry_data = voter_registry.try_to_vec().map_err(|_| ProgramError::InvalidAccountData)?;
    voter_registry_account.data.borrow_mut()[..voter_registry_data.len()].copy_from_slice(&voter_registry_data);

    // Process fee transaction
    process_fee_transaction(accounts, tx_hex)?;

    // Log success
    msg!("Vote cast successfully for poll {}: option {}", poll_id, option_index);
    
    Ok(())
}

fn process_change_vote(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    poll_id: u64,
    new_option_index: u8,
    new_encrypted_data: Option<Vec<u8>>,
    new_zk_proof: Option<Vec<u8>>,
    new_nonce: Option<Vec<u8>>,
    tx_hex: Vec<u8>,
) -> Result<(), ProgramError> {
    // Validate accounts
    if accounts.len() < 4 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    let account_iter = &mut accounts.iter();
    
    // Get accounts
    let voter_account = next_account_info(account_iter)?;
    let vote_account = next_account_info(account_iter)?;
    let poll_account = next_account_info(account_iter)?;
    let vote_count_account = next_account_info(account_iter)?;

    // Verify account permissions
    if !voter_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    
    if !vote_account.is_writable || !poll_account.is_writable || !vote_count_account.is_writable {
        return Err(ProgramError::InvalidAccountData);
    }

    // Deserialize poll data
    let poll = Poll::try_from_slice(&poll_account.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    // Verify poll exists, is active, and allows revoting
    if poll.id != poll_id {
        return Err(VotingError::PollDoesNotExist.into());
    }

    if !poll.is_active {
        return Err(VotingError::PollNotActive.into());
    }

    if !poll.allow_revote {
        return Err(VotingError::RevotingNotAllowed.into());
    }

    // Verify poll timing
    let current_time = get_bitcoin_block_height() as u64;
    
    if current_time < poll.start_time {
        return Err(VotingError::PollNotStarted.into());
    }
    
    if current_time > poll.end_time {
        return Err(VotingError::PollEnded.into());
    }

    // Verify option index is valid
    if new_option_index as usize >= poll.options.len() {
        return Err(VotingError::InvalidOptionIndex.into());
    }

    // Deserialize existing vote
    let mut vote = Vote::try_from_slice(&vote_account.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    // Verify vote belongs to this voter and poll
    if vote.voter != *voter_account.key {
        return Err(ProgramError::InvalidAccountData);
    }

    if vote.poll_id != poll_id {
        return Err(ProgramError::InvalidAccountData);
    }

    // Save old option for vote count update
    let old_option_index = vote.option_index;

    // Update vote
    vote.option_index = new_option_index;
    vote.timestamp = current_time;
    vote.encrypted_data = new_encrypted_data;
    vote.zk_proof = new_zk_proof;
    vote.nonce = new_nonce;

    // Verify zero-knowledge proof if poll is private
    if poll.is_private {
        if let Some(proof) = &vote.zk_proof {
            // In a real implementation, we would verify the ZK proof here
            // For simplicity, we'll just check that a proof was provided
            if proof.is_empty() {
                return Err(VotingError::InvalidZkProof.into());
            }
        } else {
            return Err(VotingError::InvalidZkProof.into());
        }
    }

    // Verify encryption if poll is encrypted
    if poll.is_encrypted {
        if vote.encrypted_data.is_none() {
            return Err(VotingError::InvalidEncryption.into());
        }
        
        if vote.nonce.is_none() {
            return Err(VotingError::MissingNonce.into());
        }
    }

    // Serialize and save updated vote data
    let vote_data = vote.try_to_vec().map_err(|_| ProgramError::InvalidAccountData)?;
    vote_account.data.borrow_mut()[..vote_data.len()].copy_from_slice(&vote_data);

    // Update vote count (decrement old option, increment new option)
    update_vote_count_change(
        vote_count_account,
        old_option_index,
        new_option_index,
        vote.weight,
    )?;

    // Process fee transaction
    process_fee_transaction(accounts, tx_hex)?;

    // Log success
    msg!("Vote changed successfully for poll {}: option {} -> {}", poll_id, old_option_index, new_option_index);
    
    Ok(())
}

fn process_close_poll(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    poll_id: u64,
    tx_hex: Vec<u8>,
) -> Result<(), ProgramError> {
    // Validate accounts
    if accounts.len() < 3 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    let account_iter = &mut accounts.iter();
    
    // Get accounts
    let caller_account = next_account_info(account_iter)?;
    let poll_account = next_account_info(account_iter)?;
    let vote_count_account = next_account_info(account_iter)?;

    // Verify account permissions
    if !caller_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    
    if !poll_account.is_writable || !vote_count_account.is_writable {
        return Err(ProgramError::InvalidAccountData);
    }

    // Deserialize poll data
    let mut poll = Poll::try_from_slice(&poll_account.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    // Verify poll exists and is active
    if poll.id != poll_id {
        return Err(VotingError::PollDoesNotExist.into());
    }

    if !poll.is_active {
        return Err(VotingError::PollNotActive.into());
    }

    // Check if poll can be closed
    let current_time = get_bitcoin_block_height() as u64;
    
    // Poll can be closed by anyone after end time
    // Or by the creator at any time
    if current_time <= poll.end_time && poll.creator != *caller_account.key {
        return Err(VotingError::NotPollCreator.into());
    }

    // Update poll status
    poll.is_active = false;

    // Finalize vote count if not encrypted
    if !poll.is_encrypted {
        let mut vote_count = VoteCount::try_from_slice(&vote_count_account.data.borrow())
            .map_err(|_| ProgramError::InvalidAccountData)?;
        
        vote_count.is_finalized = true;
        vote_count.last_updated = current_time;
        
        // Serialize and save updated vote count
        let vote_count_data = vote_count.try_to_vec().map_err(|_| ProgramError::InvalidAccountData)?;
        vote_count_account.data.borrow_mut()[..vote_count_data.len()].copy_from_slice(&vote_count_data);
    }

    // Serialize and save updated poll data
    let poll_data = poll.try_to_vec().map_err(|_| ProgramError::InvalidAccountData)?;
    poll_account.data.borrow_mut()[..poll_data.len()].copy_from_slice(&poll_data);

    // Process fee transaction
    process_fee_transaction(accounts, tx_hex)?;

    // Log success
    msg!("Poll closed successfully: {}", poll_id);
    
    Ok(())
}

fn process_get_results(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    poll_id: u64,
) -> Result<(), ProgramError> {
    // Validate accounts
    if accounts.len() < 2 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    let account_iter = &mut accounts.iter();
    
    // Get accounts
    let poll_account = next_account_info(account_iter)?;
    let vote_count_account = next_account_info(account_iter)?;

    // Deserialize poll data
    let poll = Poll::try_from_slice(&poll_account.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    // Verify poll exists
    if poll.id != poll_id {
        return Err(VotingError::PollDoesNotExist.into());
    }

    // Deserialize vote count
    let vote_count = VoteCount::try_from_slice(&vote_count_account.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    // Verify vote count belongs to this poll
    if vote_count.poll_id != poll_id {
        return Err(ProgramError::InvalidAccountData);
    }

    // Check if results are available
    let current_time = get_bitcoin_block_height() as u64;
    
    // For encrypted polls, results are only available after decryption
    if poll.is_encrypted && !vote_count.is_finalized {
        msg!("Results for encrypted poll {} are not yet available", poll_id);
        return Ok(());
    }

    // Log results
    msg!("Poll results for poll {}", poll_id);
    msg!("Title: {}", poll.title);
    msg!("Total voters: {}", vote_count.total_voters);
    
    for (i, option) in poll.options.iter().enumerate() {
        let count = if i < vote_count.counts.len() { vote_count.counts[i] } else { 0 };
        let percentage = if vote_count.total_voters > 0 {
            (count as f64 / vote_count.total_voters as f64) * 100.0
        } else {
            0.0
        };
        
        msg!("Option {}: {} - {} votes ({}%)", i, option, count, percentage);
    }
    
    msg!("Poll status: {}", if poll.is_active { "Active" } else { "Closed" });
    msg!("Results finalized: {}", vote_count.is_finalized);
    
    Ok(())
}

fn process_decrypt_results(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    poll_id: u64,
    decryption_key: Vec<u8>,
    tx_hex: Vec<u8>,
) -> Result<(), ProgramError> {
    // Validate accounts
    if accounts.len() < 3 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    let account_iter = &mut accounts.iter();
    
    // Get accounts
    let creator_account = next_account_info(account_iter)?;
    let poll_account = next_account_info(account_iter)?;
    let vote_count_account = next_account_info(account_iter)?;
    
    // Get vote accounts (remaining accounts)
    let vote_accounts: Vec<&AccountInfo> = account_iter.collect();

    // Verify account permissions
    if !creator_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    
    if !poll_account.is_writable || !vote_count_account.is_writable {
        return Err(ProgramError::InvalidAccountData);
    }
    
    for vote_account in &vote_accounts {
        if !vote_account.is_writable {
            return Err(ProgramError::InvalidAccountData);
        }
    }

    // Deserialize poll data
    let mut poll = Poll::try_from_slice(&poll_account.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    // Verify poll exists, is encrypted, and creator is authorized
    if poll.id != poll_id {
        return Err(VotingError::PollDoesNotExist.into());
    }

    if !poll.is_encrypted {
        return Err(VotingError::PollNotEncrypted.into());
    }

    if poll.creator != *creator_account.key {
        return Err(VotingError::NotPollCreator.into());
    }

    // Verify poll is closed
    if poll.is_active {
        let current_time = get_bitcoin_block_height() as u64;
        if current_time <= poll.end_time {
            return Err(VotingError::PollStillActive.into());
        }
        
        // Auto-close the poll if it's past end time
        poll.is_active = false;
    }

    // Deserialize vote count
    let mut vote_count = VoteCount::try_from_slice(&vote_count_account.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    // Verify vote count belongs to this poll and is not already finalized
    if vote_count.poll_id != poll_id {
        return Err(ProgramError::InvalidAccountData);
    }

    if vote_count.is_finalized {
        return Err(VotingError::ResultsAlreadyFinalized.into());
    }

    // Store decryption key in poll
    poll.decryption_key = Some(decryption_key.clone());

    // In a real implementation, we would:
    // 1. Decrypt all encrypted votes using the decryption key
    // 2. Verify the decrypted votes against any ZK proofs
    // 3. Recount all votes to ensure accuracy
    
    // For simplicity, we'll just mark the results as finalized
    vote_count.is_finalized = true;
    vote_count.last_updated = get_bitcoin_block_height() as u64;

    // Serialize and save updated poll data
    let poll_data = poll.try_to_vec().map_err(|_| ProgramError::InvalidAccountData)?;
    poll_account.data.borrow_mut()[..poll_data.len()].copy_from_slice(&poll_data);

    // Serialize and save updated vote count
    let vote_count_data = vote_count.try_to_vec().map_err(|_| ProgramError::InvalidAccountData)?;
    vote_count_account.data.borrow_mut()[..vote_count_data.len()].copy_from_slice(&vote_count_data);

    // Process fee transaction
    process_fee_transaction(accounts, tx_hex)?;

    // Log success
    msg!("Poll results decrypted successfully: {}", poll_id);
    
    Ok(())
}

fn process_delegate_vote(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    poll_id: Option<u64>,
    expiration: Option<u64>,
    tx_hex: Vec<u8>,
) -> Result<(), ProgramError> {
    // Validate accounts
    if accounts.len() < 3 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    let account_iter = &mut accounts.iter();
    
    // Get accounts
    let delegator_account = next_account_info(account_iter)?;
    let delegation_account = next_account_info(account_iter)?;
    let delegate_account = next_account_info(account_iter)?;

    // Verify account permissions
    if !delegator_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    
    if !delegation_account.is_writable {
        return Err(ProgramError::InvalidAccountData);
    }

    // Validate expiration if provided
    if let Some(exp) = expiration {
        let current_time = get_bitcoin_block_height() as u64;
        if exp <= current_time {
            return Err(VotingError::InvalidPollParameters.into());
        }
    }

    // Generate a unique delegation ID
    let current_time = get_bitcoin_block_height() as u64;
    let delegation_id = current_time.wrapping_add(delegator_account.key.to_bytes()[0] as u64);

    // Create delegation
    let delegation = Delegation {
        id: delegation_id,
        delegator: *delegator_account.key,
        delegate: *delegate_account.key,
        poll_id,
        expiration,
        is_active: true,
    };

    // Serialize and save delegation data
    let delegation_data = delegation.try_to_vec().map_err(|_| ProgramError::InvalidAccountData)?;
    
    // Ensure account has enough space
    let delegation_data_len = delegation_account.data.borrow().len();
    if delegation_data.len() > delegation_data_len {
        delegation_account.realloc(delegation_data.len(), false)?;
    }
    
    delegation_account.data.borrow_mut()[..delegation_data.len()].copy_from_slice(&delegation_data);

    // Process fee transaction
    process_fee_transaction(accounts, tx_hex)?;

    // Log success
    msg!("Vote delegation created successfully with ID: {}", delegation_id);
    
    Ok(())
}

fn process_revoke_delegation(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    delegation_id: u64,
    tx_hex: Vec<u8>,
) -> Result<(), ProgramError> {
    // Validate accounts
    if accounts.len() < 2 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    let account_iter = &mut accounts.iter();
    
    // Get accounts
    let delegator_account = next_account_info(account_iter)?;
    let delegation_account = next_account_info(account_iter)?;

    // Verify account permissions
    if !delegator_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    
    if !delegation_account.is_writable {
        return Err(ProgramError::InvalidAccountData);
    }

    // Deserialize delegation data
    let mut delegation = Delegation::try_from_slice(&delegation_account.data.borrow())
        .map_err(|_| VotingError::DelegationNotFound)?;

    // Verify delegation exists and delegator is authorized
    if delegation.id != delegation_id {
        return Err(VotingError::DelegationNotFound.into());
    }

    if delegation.delegator != *delegator_account.key {
        return Err(VotingError::NotDelegator.into());
    }

    // Update delegation status
    delegation.is_active = false;

    // Serialize and save updated delegation data
    let delegation_data = delegation.try_to_vec().map_err(|_| ProgramError::InvalidAccountData)?;
    delegation_account.data.borrow_mut()[..delegation_data.len()].copy_from_slice(&delegation_data);

    // Process fee transaction
    process_fee_transaction(accounts, tx_hex)?;

    // Log success
    msg!("Delegation revoked successfully: {}", delegation_id);
    
    Ok(())
}

fn process_update_token_balance(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    token: Pubkey,
    amount: u64,
    tx_hex: Vec<u8>,
) -> Result<(), ProgramError> {
    // Validate accounts
    if accounts.len() < 3 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    let account_iter = &mut accounts.iter();
    
    // Get accounts
    let owner_account = next_account_info(account_iter)?;
    let token_balance_account = next_account_info(account_iter)?;
    let token_account = next_account_info(account_iter)?;

    // Verify account permissions
    if !owner_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    
    if !token_balance_account.is_writable {
        return Err(ProgramError::InvalidAccountData);
    }

    // Verify token account
    // In a real implementation, we would verify the token account belongs to the owner
    // and contains the specified amount of tokens
    
    // Create or update token balance
    let token_balance = TokenBalance {
        owner: *owner_account.key,
        token,
        amount,
        last_updated: get_bitcoin_block_height() as u64,
    };

    // Serialize and save token balance data
    let token_balance_data = token_balance.try_to_vec().map_err(|_| ProgramError::InvalidAccountData)?;
    
    // Ensure account has enough space
    let token_balance_data_len = token_balance_account.data.borrow().len();
    if token_balance_data.len() > token_balance_data_len {
        token_balance_account.realloc(token_balance_data.len(), false)?;
    }
    
    token_balance_account.data.borrow_mut()[..token_balance_data.len()].copy_from_slice(&token_balance_data);

    // Process fee transaction
    process_fee_transaction(accounts, tx_hex)?;

    // Log success
    msg!("Token balance updated successfully: {} tokens for {}", amount, token);
    
    Ok(())
}

// Helper Functions

fn process_fee_transaction(
    accounts: &[AccountInfo],
    tx_hex: Vec<u8>,
) -> Result<(), ProgramError> {
    // Deserialize fee transaction
    let fees_tx: Transaction = bitcoin::consensus::deserialize(&tx_hex)
        .map_err(|_| VotingError::InvalidFeeTransaction)?;
    
    // Create a new Bitcoin transaction for our state update
    let mut tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![],
        output: vec![],
    };
    
    // Add state transitions for all writable accounts
    for account in accounts.iter().filter(|a| a.is_writable) {
        add_state_transition(&mut tx, account);
    }
    
    // Add fee input
    if fees_tx.input.is_empty() {
        return Err(VotingError::InsufficientFees.into());
    }
    
    tx.input.push(fees_tx.input[0].clone());
    
    // Prepare transaction for signing
    let tx_to_sign = TransactionToSign {
        tx_bytes: &bitcoin::consensus::serialize(&tx),
        inputs_to_sign: &[InputToSign {
            index: 0,
            signer: accounts[0].key.clone(), // First account is always the signer
        }],
    };
    
    // Submit transaction to be signed
    set_transaction_to_sign(accounts, tx_to_sign)
}

fn find_voter_index(registry: &VoterRegistry, voter: &Pubkey) -> Option<usize> {
    // First check bitmap for efficiency
    let voter_hash = hash_pubkey(voter);
    let byte_index = (voter_hash % 8192) / 8;
    let bit_index = (voter_hash % 8192) % 8;
    
    if byte_index < registry.voter_bitmap.len() as u64 {
        let byte = registry.voter_bitmap[byte_index as usize];
        if (byte & (1 << bit_index)) == 0 {
            // Bit not set, voter definitely not in registry
            return None;
        }
    } else {
        // Bitmap not large enough, voter definitely not in registry
        return None;
    }
    
    // Bit is set, check actual voters list to handle collisions
    registry.voters.iter().position(|v| v == voter)
}

fn add_voter_to_registry(registry: &mut VoterRegistry, voter: &Pubkey) -> Result<(), ProgramError> {
    // Set bit in bitmap
    let voter_hash = hash_pubkey(voter);
    let byte_index = (voter_hash % 8192) / 8;
    let bit_index = (voter_hash % 8192) % 8;
    
    if byte_index < registry.voter_bitmap.len() as u64 {
        registry.voter_bitmap[byte_index as usize] |= 1 << bit_index;
    } else {
        // Resize bitmap if needed
        while byte_index >= registry.voter_bitmap.len() as u64 {
            registry.voter_bitmap.push(0);
        }
        registry.voter_bitmap[byte_index as usize] |= 1 << bit_index;
    }
    
    // Add to voters list
    registry.voters.push(*voter);
    
    Ok(())
}

fn hash_pubkey(pubkey: &Pubkey) -> u64 {
    // Simple hash function for demo purposes
    // In production, use a proper hash function
    let bytes = pubkey.to_bytes();
    let mut hash: u64 = 0;
    
    for i in 0..8 {
        hash = hash.wrapping_add((bytes[i] as u64).wrapping_shl(i as u32 * 8));
    }
    
    hash
}

fn update_vote_count(
    vote_count_account: &AccountInfo,
    option_index: u8,
    weight: u64,
    is_revote: bool,
    previous_option: Option<u8>,
) -> Result<(), ProgramError> {
    // Deserialize vote count
    let mut vote_count = VoteCount::try_from_slice(&vote_count_account.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;
    
    // Update counts
    if is_revote {
        if let Some(prev_option) = previous_option {
            // Decrement previous option count
            if prev_option as usize < vote_count.counts.len() {
                vote_count.counts[prev_option as usize] = vote_count.counts[prev_option as usize].saturating_sub(weight);
            }
        }
    } else {
        // Increment total voters for new votes
        vote_count.total_voters = vote_count.total_voters.saturating_add(1);
    }
    
    // Increment new option count
    if option_index as usize < vote_count.counts.len() {
        vote_count.counts[option_index as usize] = vote_count.counts[option_index as usize].saturating_add(weight);
    }
    
    // Update timestamp
    vote_count.last_updated = get_bitcoin_block_height() as u64;
    
    // Serialize and save updated vote count
    let vote_count_data = vote_count.try_to_vec().map_err(|_| ProgramError::InvalidAccountData)?;
    vote_count_account.data.borrow_mut()[..vote_count_data.len()].copy_from_slice(&vote_count_data);
    
    Ok(())
}

fn update_vote_count_change(
    vote_count_account: &AccountInfo,
    old_option_index: u8,
    new_option_index: u8,
    weight: u64,
) -> Result<(), ProgramError> {
    // Deserialize vote count
    let mut vote_count = VoteCount::try_from_slice(&vote_count_account.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;
    
    // Decrement old option count
    if old_option_index as usize < vote_count.counts.len() {
        vote_count.counts[old_option_index as usize] = vote_count.counts[old_option_index as usize].saturating_sub(weight);
    }
    
    // Increment new option count
    if new_option_index as usize < vote_count.counts.len() {
        vote_count.counts[new_option_index as usize] = vote_count.counts[new_option_index as usize].saturating_add(weight);
    }
    
    // Update timestamp
    vote_count.last_updated = get_bitcoin_block_height() as u64;
    
    // Serialize and save updated vote count
    let vote_count_data = vote_count.try_to_vec().map_err(|_| ProgramError::InvalidAccountData)?;
    vote_count_account.data.borrow_mut()[..vote_count_data.len()].copy_from_slice(&vote_count_data);
    
    Ok(())
}

fn get_previous_vote(
    vote_account: &AccountInfo,
    poll_id: u64,
    voter: &Pubkey,
) -> Option<u64> {
    // Try to deserialize existing vote
    match Vote::try_from_slice(&vote_account.data.borrow()) {
        Ok(vote) => {
            // Verify vote belongs to this voter and poll
            if vote.voter == *voter && vote.poll_id == poll_id {
                Some(vote.option_index as u64)
            } else {
                None
            }
        }
        Err(_) => None,
    }
}

pub fn create_poll(title: String, options: Vec<String>, end_time: u64) {
    // Logic for creating a decentralized poll
}
