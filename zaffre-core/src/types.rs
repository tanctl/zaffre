//! core types for zaffre

use borsh::{BorshDeserialize, BorshSerialize};
use getrandom::getrandom;
use zeroize::Zeroize;

#[derive(Clone)]
pub struct Secret([u8; 32]);

impl Secret {
    pub fn generate() -> Self {
        let mut bytes = [0u8; 32];
        getrandom(&mut bytes).expect("getrandom failed");
        Self(bytes)
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl Zeroize for Secret {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        self.zeroize();
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, BorshSerialize, BorshDeserialize)]
pub struct Commitment(pub [u8; 32]);

impl Commitment {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl AsRef<[u8]> for Commitment {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, BorshSerialize, BorshDeserialize)]
pub struct Nullifier(pub [u8; 32]);

impl Nullifier {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl AsRef<[u8]> for Nullifier {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_generation() {
        let s1 = Secret::generate();
        let s2 = Secret::generate();
        assert_ne!(s1.as_bytes(), s2.as_bytes());
    }

    #[test]
    fn test_secret_from_bytes() {
        let bytes = [42u8; 32];
        let secret = Secret::from_bytes(bytes);
        assert_eq!(secret.as_bytes(), &bytes);
    }

    #[test]
    fn test_commitment_borsh() {
        let commitment = Commitment::from_bytes([1u8; 32]);
        let serialized = borsh::to_vec(&commitment).unwrap();
        let deserialized: Commitment = borsh::from_slice(&serialized).unwrap();
        assert_eq!(commitment, deserialized);
    }

    #[test]
    fn test_nullifier_borsh() {
        let nullifier = Nullifier::from_bytes([2u8; 32]);
        let serialized = borsh::to_vec(&nullifier).unwrap();
        let deserialized: Nullifier = borsh::from_slice(&serialized).unwrap();
        assert_eq!(nullifier, deserialized);
    }
}
