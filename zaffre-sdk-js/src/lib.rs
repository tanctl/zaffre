use js_sys::Uint8Array;
use sha2::{Digest, Sha256};
use wasm_bindgen::prelude::*;

use zaffre_prover::{compute_commitment, compute_nullifier, Secret};

const PUBLIC_INPUT_BYTES: usize = 160;
const PUBLIC_INPUT_COUNT: usize = PUBLIC_INPUT_BYTES;
const PUBLIC_WITNESS_HEADER_LEN: usize = 12;
const PUBLIC_WITNESS_SIZE: usize = PUBLIC_WITNESS_HEADER_LEN + PUBLIC_INPUT_COUNT * 32;
const ACTION_PARAMS_MAX_LEN: u32 = 32;

#[wasm_bindgen]
pub struct ZaffreProver;

#[wasm_bindgen]
impl ZaffreProver {
    #[wasm_bindgen(js_name = init)]
    pub fn init() -> ZaffreProver {
        ZaffreProver
    }

    #[wasm_bindgen(js_name = generateSecret)]
    pub fn generate_secret(&self) -> Result<Uint8Array, JsValue> {
        let mut bytes = [0u8; 32];
        getrandom::getrandom(&mut bytes)
            .map_err(|e| JsValue::from_str(&format!("getrandom failed: {e}")))?;
        Ok(Uint8Array::from(bytes.as_slice()))
    }

    #[wasm_bindgen(js_name = computeCommitment)]
    pub fn compute_commitment(&self, secret: Uint8Array) -> Result<Uint8Array, JsValue> {
        let secret_bytes = read_32(&secret, "secret")?;
        let commitment = compute_commitment(&Secret::from_bytes(secret_bytes));
        Ok(Uint8Array::from(commitment.as_bytes().as_slice()))
    }

    #[wasm_bindgen(js_name = computeNullifier)]
    pub fn compute_nullifier(
        &self,
        secret: Uint8Array,
        program_id: Uint8Array,
        action_id: Uint8Array,
    ) -> Result<Uint8Array, JsValue> {
        let secret_bytes = read_32(&secret, "secret")?;
        let program_bytes = read_32(&program_id, "programId")?;
        let action_bytes = read_32(&action_id, "actionId")?;
        let nullifier = compute_nullifier(
            &Secret::from_bytes(secret_bytes),
            &program_bytes,
            &action_bytes,
        );
        Ok(Uint8Array::from(nullifier.as_bytes().as_slice()))
    }

    #[wasm_bindgen(js_name = computeActionHash)]
    pub fn compute_action_hash(
        &self,
        program_id: Uint8Array,
        pda: Uint8Array,
        action_discriminator: Uint8Array,
        action_params: Uint8Array,
        nonce: u64,
    ) -> Result<Uint8Array, JsValue> {
        let program_bytes = read_32(&program_id, "programId")?;
        let pda_bytes = read_32(&pda, "pda")?;
        let discriminator = read_8(&action_discriminator, "actionDiscriminator")?;
        if action_params.length() > ACTION_PARAMS_MAX_LEN {
            return Err(JsValue::from_str(
                "actionParams must be <= 32 bytes for the ownership circuit",
            ));
        }
        let params = action_params.to_vec();

        let mut hasher = Sha256::new();
        hasher.update(program_bytes);
        hasher.update(pda_bytes);
        hasher.update(discriminator);
        hasher.update(params);
        hasher.update(nonce.to_le_bytes());
        let digest = hasher.finalize();

        Ok(Uint8Array::from(digest.as_slice()))
    }

    #[wasm_bindgen(js_name = serializePublicInputs)]
    pub fn serialize_public_inputs(
        &self,
        commitment: Uint8Array,
        pda: Uint8Array,
        program_id: Uint8Array,
        action_hash: Uint8Array,
        nullifier: Uint8Array,
    ) -> Result<Uint8Array, JsValue> {
        let commitment_bytes = read_32(&commitment, "commitment")?;
        let pda_bytes = read_32(&pda, "pda")?;
        let program_bytes = read_32(&program_id, "programId")?;
        let action_bytes = read_32(&action_hash, "actionHash")?;
        let nullifier_bytes = read_32(&nullifier, "nullifier")?;

        let mut raw = [0u8; PUBLIC_INPUT_BYTES];
        raw[0..32].copy_from_slice(&commitment_bytes);
        raw[32..64].copy_from_slice(&pda_bytes);
        raw[64..96].copy_from_slice(&program_bytes);
        raw[96..128].copy_from_slice(&action_bytes);
        raw[128..PUBLIC_INPUT_BYTES].copy_from_slice(&nullifier_bytes);

        let mut witness = Vec::with_capacity(PUBLIC_WITNESS_SIZE);
        witness.extend_from_slice(&(PUBLIC_INPUT_COUNT as u32).to_be_bytes());
        witness.extend_from_slice(&0u32.to_be_bytes());
        witness.extend_from_slice(&(PUBLIC_INPUT_COUNT as u32).to_be_bytes());
        for b in raw {
            witness.extend_from_slice(&[0u8; 31]);
            witness.push(b);
        }

        Ok(Uint8Array::from(witness.as_slice()))
    }
}

fn read_32(arr: &Uint8Array, name: &str) -> Result<[u8; 32], JsValue> {
    if arr.length() != 32 {
        return Err(JsValue::from_str(&format!(
            "{name} must be 32 bytes (got {})",
            arr.length()
        )));
    }
    let mut out = [0u8; 32];
    arr.copy_to(&mut out);
    Ok(out)
}

fn read_8(arr: &Uint8Array, name: &str) -> Result<[u8; 8], JsValue> {
    if arr.length() != 8 {
        return Err(JsValue::from_str(&format!(
            "{name} must be 8 bytes (got {})",
            arr.length()
        )));
    }
    let mut out = [0u8; 8];
    arr.copy_to(&mut out);
    Ok(out)
}
