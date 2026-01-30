//! core primitives for zaffre

pub mod encoding;
pub mod pda;
pub mod types;

pub use encoding::{
    compute_action_hash, compute_domain_separator, extract_public_inputs, serialize_public_inputs,
    serialize_public_witness, PublicInputs, PUBLIC_INPUT_BYTES, PUBLIC_WITNESS_SIZE,
};
pub use pda::{
    derive_nullifier_pda, derive_zaffre_pda, ZaffrePDA, NULLIFIER_SEED_PREFIX, ZAFFRE_SEED_PREFIX,
};
pub use types::{Commitment, Nullifier, Secret};

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::pubkey::Pubkey;

    #[test]
    fn test_full_workflow() {
        let commitment = Commitment::from_bytes([1u8; 32]);

        let program_id = Pubkey::new_unique();
        let pda = derive_zaffre_pda(&program_id, &commitment);

        let action_hash = compute_action_hash(
            &program_id,
            &pda.address,
            b"test\0\0\0\0",
            &[],
            12345,
        );
        let nullifier = Nullifier::from_bytes([9u8; 32]);

        let public_inputs = serialize_public_inputs(
            commitment.as_bytes(),
            &pda.address,
            &program_id,
            &action_hash,
            nullifier.as_bytes(),
        );
        let public_witness = serialize_public_witness(
            commitment.as_bytes(),
            &pda.address,
            &program_id,
            &action_hash,
            nullifier.as_bytes(),
        );

        assert_eq!(public_inputs.len(), PUBLIC_INPUT_BYTES);
        assert_eq!(public_witness.len(), PUBLIC_WITNESS_SIZE);
    }

    #[test]
    fn test_nullifier_workflow() {
        let program_id = Pubkey::new_unique();
        let nullifier = Nullifier::from_bytes([2u8; 32]);

        let (nullifier_pda, _bump) = derive_nullifier_pda(&program_id, &nullifier);
        assert_ne!(nullifier_pda, Pubkey::default());
    }
}
