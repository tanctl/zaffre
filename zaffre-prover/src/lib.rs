//! off-chain prover utilities

pub mod commitment;
pub mod encoding;
pub mod proof;
pub mod types;

pub use commitment::{compute_commitment, compute_domain, compute_nullifier};
pub use encoding::{bytes_to_field, field_to_bytes, is_canonical};
pub use proof::{generate_proof, ProofResult, ProverError};
pub use types::{Commitment, Nullifier, Secret};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commitment_determinism() {
        let secret = Secret::from_bytes([42u8; 32]);
        let c1 = compute_commitment(&secret);
        let c2 = compute_commitment(&secret);
        assert_eq!(c1, c2);
    }

    #[test]
    fn test_nullifier_determinism() {
        let secret = Secret::from_bytes([42u8; 32]);
        let program_id = [1u8; 32];
        let action_id = [2u8; 32];

        let n1 = compute_nullifier(&secret, &program_id, &action_id);
        let n2 = compute_nullifier(&secret, &program_id, &action_id);
        assert_eq!(n1, n2);
    }

    #[test]
    fn test_different_secrets_different_commitments() {
        let s1 = Secret::from_bytes([1u8; 32]);
        let s2 = Secret::from_bytes([2u8; 32]);
        let c1 = compute_commitment(&s1);
        let c2 = compute_commitment(&s2);
        assert_ne!(c1, c2);
    }

    #[test]
    fn test_field_encoding_roundtrip() {
        let bytes = [42u8; 32];
        let field = bytes_to_field(&bytes);
        let recovered = field_to_bytes(&field);

        let field2 = bytes_to_field(&recovered);
        let recovered2 = field_to_bytes(&field2);
        assert_eq!(recovered, recovered2);
    }
}
