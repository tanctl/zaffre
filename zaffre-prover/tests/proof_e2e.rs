use std::fs;
use std::path::PathBuf;
use std::process::Command;

use sha2::{Digest, Sha256};

use zaffre_prover::{compute_commitment, compute_nullifier, generate_proof, Secret};

struct FileBackup {
    original: PathBuf,
    backup: Option<PathBuf>,
}

impl FileBackup {
    fn new(original: PathBuf) -> Self {
        if original.exists() {
            let backup = original.with_extension(format!(
                "bak.{}",
                std::process::id()
            ));
            let _ = fs::copy(&original, &backup);
            return Self {
                original,
                backup: Some(backup),
            };
        }
        Self {
            original,
            backup: None,
        }
    }
}

impl Drop for FileBackup {
    fn drop(&mut self) {
        if let Some(backup) = self.backup.take() {
            let _ = fs::copy(&backup, &self.original);
            let _ = fs::remove_file(&backup);
        } else {
            let _ = fs::remove_file(&self.original);
        }
    }
}

#[test]
#[ignore]
fn e2e_generate_and_verify_proof() {
    if std::env::var("ZAFFRE_E2E").is_err() {
        eprintln!("Skipping: set ZAFFRE_E2E=1 to run end-to-end proof test.");
        return;
    }

    let circuit_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("circuits")
        .join("ownership");

    let secret = Secret::from_bytes([7u8; 32]);
    let commitment = compute_commitment(&secret);

    let program_id = [3u8; 32];
    let pda = [4u8; 32];

    let action_discriminator = *b"setvalue";
    let value = 123u64;
    let params = value.to_le_bytes();
    let nonce = 42u64;

    let mut hasher = Sha256::new();
    hasher.update(program_id);
    hasher.update(pda);
    hasher.update(action_discriminator);
    hasher.update(params);
    hasher.update(nonce.to_le_bytes());
    let action_hash: [u8; 32] = hasher.finalize().into();
    let nullifier = compute_nullifier(&secret, &program_id, &action_hash);

    let circuit_name = "ownership";
    let proof_path = circuit_dir
        .join("target")
        .join(format!("{circuit_name}.proof"));
    let public_witness_path = circuit_dir
        .join("target")
        .join(format!("{circuit_name}.pw"));
    let _proof_backup = FileBackup::new(proof_path);
    let _public_witness_backup = FileBackup::new(public_witness_path);

    let result = generate_proof(
        &secret,
        &commitment,
        &pda,
        &program_id,
        &action_hash,
        &nullifier,
        &params,
        nonce,
        &circuit_dir,
    )
    .expect("proof generation failed");

    let vk_path = circuit_dir
        .join("target")
        .join(format!("{circuit_name}.vk"));

    let status = Command::new(
        std::env::var_os("SUNSPOT_BIN").unwrap_or_else(|| "sunspot".into()),
    )
        .current_dir(circuit_dir.join("target"))
        .arg("verify")
        .arg(&vk_path)
        .arg(&result.proof_path)
        .arg(&result.public_witness_path)
        .status()
        .expect("failed to run sunspot verify");

    assert!(status.success(), "sunspot verify failed");
}
