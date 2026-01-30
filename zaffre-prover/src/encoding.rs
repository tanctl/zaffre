//! field encoding helpers

use ark_bn254::Fr;
use ark_ff::{BigInteger, PrimeField};

pub fn bytes_to_field(bytes: &[u8; 32]) -> Fr {
    Fr::from_le_bytes_mod_order(bytes)
}

pub fn field_to_bytes(field: &Fr) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    let repr = field.into_bigint();
    let le_bytes = repr.to_bytes_le();
    bytes[..le_bytes.len().min(32)].copy_from_slice(&le_bytes[..le_bytes.len().min(32)]);
    bytes
}

pub fn is_canonical(bytes: &[u8; 32]) -> bool {
    let field = bytes_to_field(bytes);
    let recovered = field_to_bytes(&field);
    bytes == &recovered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bytes_to_field_zero() {
        let bytes = [0u8; 32];
        let field = bytes_to_field(&bytes);
        assert_eq!(field, Fr::from(0u64));
    }

    #[test]
    fn test_bytes_to_field_one() {
        let mut bytes = [0u8; 32];
        bytes[0] = 1;
        let field = bytes_to_field(&bytes);
        assert_eq!(field, Fr::from(1u64));
    }

    #[test]
    fn test_field_to_bytes_zero() {
        let field = Fr::from(0u64);
        let bytes = field_to_bytes(&field);
        assert_eq!(bytes, [0u8; 32]);
    }

    #[test]
    fn test_field_to_bytes_one() {
        let field = Fr::from(1u64);
        let bytes = field_to_bytes(&field);
        let mut expected = [0u8; 32];
        expected[0] = 1;
        assert_eq!(bytes, expected);
    }

    #[test]
    fn test_roundtrip_small() {
        for i in 0..256u64 {
            let field = Fr::from(i);
            let bytes = field_to_bytes(&field);
            let recovered = bytes_to_field(&bytes);
            assert_eq!(field, recovered, "Failed for i={}", i);
        }
    }

    #[test]
    fn test_is_canonical() {
        let mut tiny = [0u8; 32];
        tiny[0] = 42;
        assert!(is_canonical(&tiny));
    }
}
