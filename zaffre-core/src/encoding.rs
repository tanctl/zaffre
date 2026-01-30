//! encoding helpers

use solana_program::{hash::{hash, hashv}, pubkey::Pubkey};

pub const PUBLIC_INPUT_BYTES: usize = 160;
pub const PUBLIC_INPUT_COUNT: usize = PUBLIC_INPUT_BYTES;
pub const PUBLIC_WITNESS_HEADER_LEN: usize = 12;
pub const PUBLIC_WITNESS_SIZE: usize = PUBLIC_WITNESS_HEADER_LEN + PUBLIC_INPUT_COUNT * 32;
pub const OWNERSHIP_ACTION_PARAMS_MAX_LEN: usize = 32;
pub fn serialize_public_inputs(
    commitment: &[u8; 32],
    pda: &Pubkey,
    program_id: &Pubkey,
    action_hash: &[u8; 32],
    nullifier: &[u8; 32],
) -> [u8; PUBLIC_INPUT_BYTES] {
    let mut result = [0u8; PUBLIC_INPUT_BYTES];
    result[0..32].copy_from_slice(commitment);
    result[32..64].copy_from_slice(&pda.to_bytes());
    result[64..96].copy_from_slice(&program_id.to_bytes());
    result[96..128].copy_from_slice(action_hash);
    result[128..PUBLIC_INPUT_BYTES].copy_from_slice(nullifier);
    result
}

pub fn serialize_public_witness(
    commitment: &[u8; 32],
    pda: &Pubkey,
    program_id: &Pubkey,
    action_hash: &[u8; 32],
    nullifier: &[u8; 32],
) -> Vec<u8> {
    let raw = serialize_public_inputs(commitment, pda, program_id, action_hash, nullifier);
    let mut out = vec![0u8; PUBLIC_WITNESS_SIZE];
    out[0..4].copy_from_slice(&(PUBLIC_INPUT_COUNT as u32).to_be_bytes());
    out[4..8].copy_from_slice(&0u32.to_be_bytes());
    out[8..12].copy_from_slice(&(PUBLIC_INPUT_COUNT as u32).to_be_bytes());

    let mut offset = PUBLIC_WITNESS_HEADER_LEN + 31;
    for b in raw {
        out[offset] = b;
        offset += 32;
    }
    out
}

pub fn extract_public_inputs(public_witness: &[u8]) -> Option<[u8; PUBLIC_INPUT_BYTES]> {
    if public_witness.len() != PUBLIC_WITNESS_SIZE {
        return None;
    }

    let public_count = u32::from_be_bytes(public_witness[0..4].try_into().ok()?) as usize;
    let reserved = u32::from_be_bytes(public_witness[4..8].try_into().ok()?) as usize;
    let vector_len = u32::from_be_bytes(public_witness[8..12].try_into().ok()?) as usize;
    if public_count != PUBLIC_INPUT_COUNT || reserved != 0 || vector_len != PUBLIC_INPUT_COUNT {
        return None;
    }

    let mut raw = [0u8; PUBLIC_INPUT_BYTES];
    let mut offset = PUBLIC_WITNESS_HEADER_LEN;
    for byte in raw.iter_mut() {
        *byte = public_witness[offset + 31];
        offset += 32;
    }
    Some(raw)
}

pub fn compute_action_hash(
    program_id: &Pubkey,
    pda: &Pubkey,
    action_discriminator: &[u8; 8],
    action_params: &[u8],
    nonce: u64,
) -> [u8; 32] {
    assert!(
        action_params.len() <= OWNERSHIP_ACTION_PARAMS_MAX_LEN,
        "action_params must be <= {OWNERSHIP_ACTION_PARAMS_MAX_LEN} bytes"
    );
    let nonce_bytes = nonce.to_le_bytes();
    hashv(&[
        &program_id.to_bytes(),
        &pda.to_bytes(),
        action_discriminator,
        action_params,
        &nonce_bytes,
    ])
    .to_bytes()
}

pub fn compute_domain_separator(program_id: &Pubkey) -> [u8; 32] {
    hash(&program_id.to_bytes()).to_bytes()
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicInputs {
    pub commitment: [u8; 32],
    pub pda: [u8; 32],
    pub program_id: [u8; 32],
    pub action_hash: [u8; 32],
    pub nullifier: [u8; 32],
}

impl PublicInputs {
    pub fn new(
        commitment: [u8; 32],
        pda: &Pubkey,
        program_id: &Pubkey,
        action_hash: [u8; 32],
        nullifier: [u8; 32],
    ) -> Self {
        Self {
            commitment,
            pda: pda.to_bytes(),
            program_id: program_id.to_bytes(),
            action_hash,
            nullifier,
        }
    }

    pub fn to_bytes(&self) -> [u8; PUBLIC_INPUT_BYTES] {
        let mut result = [0u8; PUBLIC_INPUT_BYTES];
        result[0..32].copy_from_slice(&self.commitment);
        result[32..64].copy_from_slice(&self.pda);
        result[64..96].copy_from_slice(&self.program_id);
        result[96..128].copy_from_slice(&self.action_hash);
        result[128..PUBLIC_INPUT_BYTES].copy_from_slice(&self.nullifier);
        result
    }

    pub fn to_public_witness(&self) -> Vec<u8> {
        serialize_public_witness(
            &self.commitment,
            &Pubkey::new_from_array(self.pda),
            &Pubkey::new_from_array(self.program_id),
            &self.action_hash,
            &self.nullifier,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_public_inputs() {
        let commitment = [1u8; 32];
        let pda = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();
        let action_hash = [2u8; 32];
        let nullifier = [3u8; 32];

        let bytes =
            serialize_public_inputs(&commitment, &pda, &program_id, &action_hash, &nullifier);

        assert_eq!(bytes.len(), PUBLIC_INPUT_BYTES);
        assert_eq!(&bytes[0..32], &commitment);
        assert_eq!(&bytes[32..64], &pda.to_bytes());
        assert_eq!(&bytes[64..96], &program_id.to_bytes());
        assert_eq!(&bytes[96..128], &action_hash);
        assert_eq!(&bytes[128..PUBLIC_INPUT_BYTES], &nullifier);
    }

    #[test]
    fn test_public_witness_roundtrip() {
        let commitment = [3u8; 32];
        let pda = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();
        let action_hash = [4u8; 32];
        let nullifier = [5u8; 32];

        let witness =
            serialize_public_witness(&commitment, &pda, &program_id, &action_hash, &nullifier);
        assert_eq!(witness.len(), PUBLIC_WITNESS_SIZE);

        let raw = extract_public_inputs(&witness).expect("witness should decode");
        assert_eq!(&raw[0..32], &commitment);
        assert_eq!(&raw[32..64], &pda.to_bytes());
        assert_eq!(&raw[64..96], &program_id.to_bytes());
        assert_eq!(&raw[96..128], &action_hash);
        assert_eq!(&raw[128..PUBLIC_INPUT_BYTES], &nullifier);
    }

    #[test]
    fn test_compute_action_hash_deterministic() {
        let program_id = Pubkey::new_unique();
        let pda = Pubkey::new_unique();
        let discriminator = *b"transfer";
        let params = [42u8; 16];
        let nonce = 12345u64;

        let hash1 = compute_action_hash(&program_id, &pda, &discriminator, &params, nonce);
        let hash2 = compute_action_hash(&program_id, &pda, &discriminator, &params, nonce);

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 32);
    }

    #[test]
    fn test_action_hash_different_nonce() {
        let program_id = Pubkey::new_unique();
        let pda = Pubkey::new_unique();
        let discriminator = *b"transfer";
        let params = [];

        let hash1 = compute_action_hash(&program_id, &pda, &discriminator, &params, 1);
        let hash2 = compute_action_hash(&program_id, &pda, &discriminator, &params, 2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    #[should_panic(expected = "action_params must be <= 32 bytes")]
    fn test_action_hash_params_too_long() {
        let program_id = Pubkey::new_unique();
        let pda = Pubkey::new_unique();
        let discriminator = *b"transfer";
        let params = vec![1u8; OWNERSHIP_ACTION_PARAMS_MAX_LEN + 1];

        let _ = compute_action_hash(&program_id, &pda, &discriminator, &params, 1);
    }

    #[test]
    fn test_public_inputs_struct() {
        let commitment = [1u8; 32];
        let pda = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();
        let action_hash = [2u8; 32];
        let nullifier = [3u8; 32];

        let inputs = PublicInputs::new(commitment, &pda, &program_id, action_hash, nullifier);
        let bytes = inputs.to_bytes();

        assert_eq!(bytes.len(), PUBLIC_INPUT_BYTES);
        assert_eq!(&bytes[0..32], &commitment);
    }

    #[test]
    fn test_public_witness_header_and_entries() {
        let commitment = [1u8; 32];
        let pda = Pubkey::new_from_array([2u8; 32]);
        let program_id = Pubkey::new_from_array([3u8; 32]);
        let action_hash = [4u8; 32];
        let nullifier = [5u8; 32];

        let witness =
            serialize_public_witness(&commitment, &pda, &program_id, &action_hash, &nullifier);
        assert_eq!(witness.len(), PUBLIC_WITNESS_SIZE);

        let expected_count = (PUBLIC_INPUT_COUNT as u32).to_be_bytes();
        assert_eq!(&witness[0..4], &expected_count);
        assert_eq!(&witness[4..8], &0u32.to_be_bytes());
        assert_eq!(&witness[8..12], &expected_count);

        let first_entry = PUBLIC_WITNESS_HEADER_LEN;
        assert!(witness[first_entry..first_entry + 31]
            .iter()
            .all(|b| *b == 0));
        assert_eq!(witness[first_entry + 31], commitment[0]);

        let action_hash_entry = PUBLIC_WITNESS_HEADER_LEN + 32 * 96;
        assert!(witness[action_hash_entry..action_hash_entry + 31]
            .iter()
            .all(|b| *b == 0));
        assert_eq!(witness[action_hash_entry + 31], action_hash[0]);

        let nullifier_entry = PUBLIC_WITNESS_HEADER_LEN + 32 * 128;
        assert!(witness[nullifier_entry..nullifier_entry + 31]
            .iter()
            .all(|b| *b == 0));
        assert_eq!(witness[nullifier_entry + 31], nullifier[0]);
    }

    #[test]
    fn test_domain_separator() {
        let program_id = Pubkey::new_unique();
        let sep1 = compute_domain_separator(&program_id);
        let sep2 = compute_domain_separator(&program_id);

        assert_eq!(sep1, sep2);
        assert_eq!(sep1.len(), 32);

        let other_program = Pubkey::new_unique();
        let sep3 = compute_domain_separator(&other_program);
        assert_ne!(sep1, sep3);
    }
}
