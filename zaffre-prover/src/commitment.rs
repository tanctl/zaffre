//! commitment and nullifier helpers

use ark_bn254::Fr;
use light_poseidon::{Poseidon, PoseidonHasher};
use sha2::{Digest, Sha256};

use crate::encoding::{bytes_to_field, field_to_bytes};
use crate::types::{Commitment, Nullifier, Secret};

pub fn compute_commitment(secret: &Secret) -> Commitment {
    let secret_field = bytes_to_field(secret.as_bytes());

    let mut poseidon = Poseidon::<Fr>::new_circom(1).expect("Poseidon init failed");
    let commitment_field = poseidon.hash(&[secret_field]).expect("Poseidon hash failed");

    let commitment_bytes = field_to_bytes(&commitment_field);
    Commitment::from_bytes(commitment_bytes)
}

pub fn compute_nullifier(
    secret: &Secret,
    program_id_bytes: &[u8; 32],
    action_id: &[u8; 32],
) -> Nullifier {
    let domain_bytes = {
        let mut hasher = Sha256::new();
        hasher.update(program_id_bytes);
        let result: [u8; 32] = hasher.finalize().into();
        result
    };

    let secret_field = bytes_to_field(secret.as_bytes());
    let domain_field = bytes_to_field(&domain_bytes);
    let action_field = bytes_to_field(action_id);

    let mut poseidon = Poseidon::<Fr>::new_circom(3).expect("Poseidon init failed");
    let nullifier_field = poseidon
        .hash(&[secret_field, domain_field, action_field])
        .expect("Poseidon hash failed");

    let nullifier_bytes = field_to_bytes(&nullifier_field);
    Nullifier::from_bytes(nullifier_bytes)
}

pub fn compute_domain(program_id_bytes: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(program_id_bytes);
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commitment_non_zero() {
        let secret = Secret::from_bytes([1u8; 32]);
        let commitment = compute_commitment(&secret);
        assert_ne!(commitment.as_bytes(), &[0u8; 32]);
    }

    #[test]
    fn test_commitment_deterministic() {
        let secret = Secret::from_bytes([42u8; 32]);
        let c1 = compute_commitment(&secret);
        let c2 = compute_commitment(&secret);
        assert_eq!(c1, c2);
    }

    #[test]
    fn test_different_secrets_different_commitments() {
        let s1 = Secret::from_bytes([1u8; 32]);
        let s2 = Secret::from_bytes([2u8; 32]);
        assert_ne!(compute_commitment(&s1), compute_commitment(&s2));
    }

    #[test]
    fn test_nullifier_non_zero() {
        let secret = Secret::from_bytes([1u8; 32]);
        let program_id = [2u8; 32];
        let action_id = [3u8; 32];
        let nullifier = compute_nullifier(&secret, &program_id, &action_id);
        assert_ne!(nullifier.as_bytes(), &[0u8; 32]);
    }

    #[test]
    fn test_nullifier_deterministic() {
        let secret = Secret::from_bytes([42u8; 32]);
        let program_id = [1u8; 32];
        let action_id = [2u8; 32];

        let n1 = compute_nullifier(&secret, &program_id, &action_id);
        let n2 = compute_nullifier(&secret, &program_id, &action_id);
        assert_eq!(n1, n2);
    }

    #[test]
    fn test_nullifier_different_actions() {
        let secret = Secret::from_bytes([42u8; 32]);
        let program_id = [1u8; 32];

        let n1 = compute_nullifier(&secret, &program_id, &[1u8; 32]);
        let n2 = compute_nullifier(&secret, &program_id, &[2u8; 32]);
        assert_ne!(n1, n2);
    }

    #[test]
    fn test_nullifier_different_programs() {
        let secret = Secret::from_bytes([42u8; 32]);
        let action_id = [1u8; 32];

        let n1 = compute_nullifier(&secret, &[1u8; 32], &action_id);
        let n2 = compute_nullifier(&secret, &[2u8; 32], &action_id);
        assert_ne!(n1, n2);
    }
}
