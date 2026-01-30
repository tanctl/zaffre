use solana_program::pubkey::Pubkey;
use zaffre_core::*;

#[test]
fn test_secret_generation() {
    let s1 = Secret::generate();
    let s2 = Secret::generate();
    assert_ne!(s1.as_bytes(), s2.as_bytes());
}

#[test]
fn test_pda_derivation() {
    let commitment = Commitment::from_bytes([1u8; 32]);
    let program_id = Pubkey::new_unique();
    let pda = derive_zaffre_pda(&program_id, &commitment);

    let pda2 = derive_zaffre_pda(&program_id, &commitment);
    assert_eq!(pda.address, pda2.address);
}

#[test]
fn test_nullifier_derivation() {
    let nullifier = Nullifier::from_bytes([1u8; 32]);
    let program_id = Pubkey::new_unique();
    let (nul_pda, _) = derive_nullifier_pda(&program_id, &nullifier);

    assert_ne!(nul_pda, Pubkey::default());
}

#[test]
fn test_action_hash_computation() {
    let program_id = Pubkey::new_unique();
    let pda = Pubkey::new_unique();
    let discriminator = *b"transfer";
    let params = [1u8, 2, 3, 4];
    let nonce = 12345u64;

    let hash1 = compute_action_hash(&program_id, &pda, &discriminator, &params, nonce);
    let hash2 = compute_action_hash(&program_id, &pda, &discriminator, &params, nonce);

    assert_eq!(hash1, hash2);

    let hash3 = compute_action_hash(&program_id, &pda, &discriminator, &params, nonce + 1);
    assert_ne!(hash1, hash3);
}

#[test]
fn test_public_inputs_serialization() {
    let commitment = [1u8; 32];
    let pda = Pubkey::new_unique();
    let program_id = Pubkey::new_unique();
    let action_hash = [2u8; 32];
    let nullifier = [3u8; 32];

    let bytes =
        serialize_public_inputs(&commitment, &pda, &program_id, &action_hash, &nullifier);

    assert_eq!(bytes.len(), 160);

    assert_eq!(&bytes[0..32], &commitment);
    assert_eq!(&bytes[32..64], &pda.to_bytes());
    assert_eq!(&bytes[64..96], &program_id.to_bytes());
    assert_eq!(&bytes[96..128], &action_hash);
    assert_eq!(&bytes[128..160], &nullifier);
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

    assert_eq!(bytes.len(), 160);
    assert_eq!(&bytes[0..32], &commitment);
}

#[test]
fn test_domain_separator() {
    let program_id = Pubkey::new_unique();
    let sep1 = compute_domain_separator(&program_id);
    let sep2 = compute_domain_separator(&program_id);

    assert_eq!(sep1, sep2);

    let other = Pubkey::new_unique();
    let sep3 = compute_domain_separator(&other);
    assert_ne!(sep1, sep3);
}
