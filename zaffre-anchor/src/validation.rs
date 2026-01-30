//! pda validation and nullifier helpers

use anchor_lang::prelude::*;

pub const ZAFFRE_SEED_PREFIX: &[u8] = b"zaffre";
pub const NULLIFIER_SEED_PREFIX: &[u8] = b"nullifier";

#[error_code]
pub enum ZaffreError {
    #[msg("PDA derivation mismatch")]
    InvalidPDA,
    #[msg("Nullifier has already been spent")]
    NullifierAlreadySpent,
}

pub fn validate_zaffre_pda(
    program_id: &Pubkey,
    commitment: &[u8; 32],
    provided_pda: &Pubkey,
) -> Result<u8> {
    let (derived, bump) =
        Pubkey::find_program_address(&[ZAFFRE_SEED_PREFIX, commitment], program_id);
    require_keys_eq!(derived, *provided_pda, ZaffreError::InvalidPDA);
    Ok(bump)
}

pub fn validate_nullifier_pda(
    program_id: &Pubkey,
    nullifier: &[u8; 32],
    provided_pda: &Pubkey,
) -> Result<u8> {
    let (derived, bump) =
        Pubkey::find_program_address(&[NULLIFIER_SEED_PREFIX, nullifier], program_id);
    require_keys_eq!(derived, *provided_pda, ZaffreError::InvalidPDA);
    Ok(bump)
}

pub const NULLIFIER_STATE_SIZE: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NullifierStateData {
    pub spent_at_slot: u64,
}

impl NullifierStateData {
    pub fn new(slot: u64) -> Self {
        Self { spent_at_slot: slot }
    }

    pub fn to_bytes(&self) -> [u8; 8] {
        self.spent_at_slot.to_le_bytes()
    }

    pub fn from_bytes(bytes: &[u8; 8]) -> Self {
        Self {
            spent_at_slot: u64::from_le_bytes(*bytes),
        }
    }
}
