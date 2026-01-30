use std::path::PathBuf;

use sha2::{Digest, Sha256};

use zaffre_prover::{compute_commitment, compute_nullifier, generate_proof, Secret};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 6 {
        eprintln!(
            "Usage: zaffre_prove <circuit_dir> <program_id_hex> <pda_hex> <value_u64> <nonce_u64> [secret_hex|random]"
        );
        std::process::exit(1);
    }

    let circuit_dir = PathBuf::from(&args[1]);
    let program_id = decode_32(&args[2], "program_id");
    let pda = decode_32(&args[3], "pda");
    let value: u64 = args[4].parse().expect("value must be u64");
    let nonce: u64 = args[5].parse().expect("nonce must be u64");

    let secret = if args.get(6).map(|s| s.as_str()) == Some("random") || args.len() == 6 {
        let mut bytes = [0u8; 32];
        getrandom::getrandom(&mut bytes).expect("getrandom failed");
        Secret::from_bytes(bytes)
    } else {
        let secret_bytes = decode_32(&args[6], "secret");
        Secret::from_bytes(secret_bytes)
    };

    let commitment = compute_commitment(&secret);

    let action_discriminator = *b"setvalue";
    let mut hasher = Sha256::new();
    hasher.update(program_id);
    hasher.update(pda);
    hasher.update(action_discriminator);
    let action_params = value.to_le_bytes();
    hasher.update(action_params);
    hasher.update(nonce.to_le_bytes());
    let action_hash: [u8; 32] = hasher.finalize().into();
    let nullifier = compute_nullifier(&secret, &program_id, &action_hash);

    let result = generate_proof(
        &secret,
        &commitment,
        &pda,
        &program_id,
        &action_hash,
        &nullifier,
        &action_params,
        nonce,
        &circuit_dir,
    )
    .expect("proof generation failed");

    println!("commitment_hex={}", hex::encode(commitment.as_bytes()));
    println!("program_id_hex={}", hex::encode(program_id));
    println!("pda_hex={}", hex::encode(pda));
    println!("value={}", value);
    println!("nonce={}", nonce);
    println!("nullifier_hex={}", hex::encode(nullifier.as_bytes()));
    println!("proof_path={}", result.proof_path.display());
    println!("public_witness_path={}", result.public_witness_path.display());
}

fn decode_32(hex_str: &str, name: &str) -> [u8; 32] {
    let bytes = hex::decode(hex_str.trim_start_matches("0x"))
        .unwrap_or_else(|_| panic!("{name} must be hex"));
    assert_eq!(bytes.len(), 32, "{name} must be 32 bytes");
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    out
}
