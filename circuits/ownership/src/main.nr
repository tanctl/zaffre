use poseidon;
use sha256::sha256_var;

global ACTION_DISCRIMINATOR: [u8; 8] = [115, 101, 116, 118, 97, 108, 117, 101];
global ACTION_PARAMS_MAX_LEN: u32 = 32;
global ACTION_HASH_BASE_LEN: u32 = 80;
global ACTION_HASH_MAX_INPUT_LEN: u32 = ACTION_HASH_BASE_LEN + ACTION_PARAMS_MAX_LEN;
global DOMAIN_HASH_INPUT_LEN: u32 = 32;

fn main(
    secret: Field,
    commitment: pub [u8; 32],
    pda: pub [u8; 32],
    program_id: pub [u8; 32],
    action_hash: pub [u8; 32],
    nullifier: pub [u8; 32],
    action_params: [u8; ACTION_PARAMS_MAX_LEN],
    action_params_len: u32,
    nonce: [u8; 8]
) {
    let computed = poseidon::poseidon::bn254::hash_1([secret]);
    let commitment_bytes = computed.to_le_bytes();
    assert(commitment_bytes == commitment, "Commitment mismatch: secret does not hash to commitment");

    assert(action_params_len <= ACTION_PARAMS_MAX_LEN, "Action params length too large");

    let computed_hash = compute_action_hash(
        program_id,
        pda,
        action_params,
        action_params_len,
        nonce,
    );
    assert(computed_hash == action_hash, "Action hash mismatch");

    let computed_nullifier = compute_nullifier(secret, program_id, action_hash);
    assert(computed_nullifier == nullifier, "Nullifier mismatch");
}

fn bytes_to_field_le(bytes: [u8; 32]) -> Field {
    let mut result: Field = 0;
    let mut multiplier: Field = 1;
    for i in 0..32 {
        result = result + (bytes[i] as Field) * multiplier;
        multiplier = multiplier * 256;
    }
    result
}

fn compute_action_hash(
    program_id: [u8; 32],
    pda: [u8; 32],
    action_params: [u8; ACTION_PARAMS_MAX_LEN],
    action_params_len: u32,
    nonce: [u8; 8]
) -> [u8; 32] {
    let mut msg: [u8; ACTION_HASH_MAX_INPUT_LEN] = [0; ACTION_HASH_MAX_INPUT_LEN];
    for i in 0..32 {
        msg[i] = program_id[i];
        msg[32 + i] = pda[i];
    }
    for i in 0..8 {
        msg[64 + i] = ACTION_DISCRIMINATOR[i];
    }
    for i in 0..ACTION_PARAMS_MAX_LEN {
        msg[72 + i] = action_params[i];
    }
    let nonce_offset = 72 + action_params_len;
    for i in 0..8 {
        msg[nonce_offset + i] = nonce[i];
    }

    let action_hash_len: u64 = (ACTION_HASH_BASE_LEN + action_params_len) as u64;
    sha256_var::<ACTION_HASH_MAX_INPUT_LEN>(msg, action_hash_len)
}

fn compute_commitment(secret: Field) -> [u8; 32] {
    let commitment_field = poseidon::poseidon::bn254::hash_1([secret]);
    commitment_field.to_le_bytes()
}

fn compute_nullifier(secret: Field, program_id: [u8; 32], action_hash: [u8; 32]) -> [u8; 32] {
    let domain_len: u64 = DOMAIN_HASH_INPUT_LEN as u64;
    let domain_bytes = sha256_var::<DOMAIN_HASH_INPUT_LEN>(program_id, domain_len);
    let domain_field = bytes_to_field_le(domain_bytes);
    let action_field = bytes_to_field_le(action_hash);
    let nullifier_field = poseidon::poseidon::bn254::hash_3([secret, domain_field, action_field]);
    nullifier_field.to_le_bytes()
}

fn u64_to_le_bytes(value: u64) -> [u8; 8] {
    let mut out: [u8; 8] = [0; 8];
    let mut v = value;
    for i in 0..8 {
        out[i] = (v & 0xff) as u8;
        v = v >> 8;
    }
    out
}

fn action_params_from_u64(value: u64) -> [u8; ACTION_PARAMS_MAX_LEN] {
    let mut out: [u8; ACTION_PARAMS_MAX_LEN] = [0; ACTION_PARAMS_MAX_LEN];
    let value_bytes: [u8; 8] = u64_to_le_bytes(value);
    for i in 0..8 {
        out[i] = value_bytes[i];
    }
    out
}

#[test]
fn test_valid_commitment() {
    let secret: Field = 1;
    let computed = poseidon::poseidon::bn254::hash_1([secret]);
    
    assert(computed != 0, "Poseidon should produce non-zero output");
}

#[test]
fn test_valid_proof_inputs() {
    let secret: Field = 7;
    let commitment = compute_commitment(secret);
    let program_id: [u8; 32] = [3; 32];
    let pda: [u8; 32] = [4; 32];
    let action_params = action_params_from_u64(9);
    let action_params_len: u32 = 8;
    let nonce: [u8; 8] = u64_to_le_bytes(11);
    let action_hash = compute_action_hash(
        program_id,
        pda,
        action_params,
        action_params_len,
        nonce,
    );
    let nullifier = compute_nullifier(secret, program_id, action_hash);

    main(
        secret,
        commitment,
        pda,
        program_id,
        action_hash,
        nullifier,
        action_params,
        action_params_len,
        nonce,
    );
}

#[test(should_fail_with = "Action hash mismatch")]
fn test_action_hash_mismatch() {
    let secret: Field = 7;
    let commitment = compute_commitment(secret);
    let program_id: [u8; 32] = [3; 32];
    let pda: [u8; 32] = [4; 32];
    let action_params = action_params_from_u64(9);
    let action_params_len: u32 = 8;
    let nonce: [u8; 8] = u64_to_le_bytes(11);
    let mut action_hash = compute_action_hash(
        program_id,
        pda,
        action_params,
        action_params_len,
        nonce,
    );
    action_hash[0] = action_hash[0] ^ 1;
    let nullifier = compute_nullifier(secret, program_id, action_hash);

    main(
        secret,
        commitment,
        pda,
        program_id,
        action_hash,
        nullifier,
        action_params,
        action_params_len,
        nonce,
    );
}

#[test(should_fail_with = "Nullifier mismatch")]
fn test_nullifier_mismatch() {
    let secret: Field = 7;
    let commitment = compute_commitment(secret);
    let program_id: [u8; 32] = [3; 32];
    let pda: [u8; 32] = [4; 32];
    let action_params = action_params_from_u64(9);
    let action_params_len: u32 = 8;
    let nonce: [u8; 8] = u64_to_le_bytes(11);
    let action_hash = compute_action_hash(
        program_id,
        pda,
        action_params,
        action_params_len,
        nonce,
    );
    let mut nullifier = compute_nullifier(secret, program_id, action_hash);
    nullifier[0] = nullifier[0] ^ 1;

    main(
        secret,
        commitment,
        pda,
        program_id,
        action_hash,
        nullifier,
        action_params,
        action_params_len,
        nonce,
    );
}

#[test(should_fail_with = "Action params length too large")]
fn test_action_params_len_too_large() {
    let secret: Field = 7;
    let commitment = compute_commitment(secret);
    let program_id: [u8; 32] = [3; 32];
    let pda: [u8; 32] = [4; 32];
    let action_params = action_params_from_u64(9);
    let action_params_len: u32 = ACTION_PARAMS_MAX_LEN + 1;
    let nonce: [u8; 8] = u64_to_le_bytes(11);
    let action_hash = compute_action_hash(
        program_id,
        pda,
        action_params,
        ACTION_PARAMS_MAX_LEN,
        nonce,
    );
    let nullifier = compute_nullifier(secret, program_id, action_hash);

    main(
        secret,
        commitment,
        pda,
        program_id,
        action_hash,
        nullifier,
        action_params,
        action_params_len,
        nonce,
    );
}
