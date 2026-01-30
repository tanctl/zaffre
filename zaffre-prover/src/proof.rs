//! proof generation via nargo and sunspot

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use num_bigint::BigUint;
use serde::Deserialize;
use thiserror::Error;
use ark_ff::{BigInteger, PrimeField};

use crate::encoding::bytes_to_field;
use crate::types::{Commitment, Nullifier, Secret};

#[derive(Debug, Error)]
pub enum ProverError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("toml parse error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("missing tool in PATH: {0}")]
    MissingTool(String),
    #[error("command failed: {cmd}\nstdout: {stdout}\nstderr: {stderr}")]
    CommandFailed {
        cmd: String,
        stdout: String,
        stderr: String,
    },
    #[error("missing expected output file: {0}")]
    MissingOutput(PathBuf),
    #[error("invalid public witness length: {0}")]
    InvalidPublicWitness(usize),
    #[error("invalid Nargo.toml: missing [package] name")]
    MissingCircuitName,
    #[error("action params too long: {0}")]
    ActionParamsTooLong(usize),
}

const ACTION_PARAMS_MAX_LEN: usize = 32;
const PUBLIC_WITNESS_SIZE: usize = 5132;

pub struct ProofResult {
    pub proof: Vec<u8>,
    pub public_witness: Vec<u8>,
    pub proof_path: PathBuf,
    pub public_witness_path: PathBuf,
}

#[derive(Debug, Deserialize)]
struct NargoManifest {
    package: Option<NargoPackage>,
}

#[derive(Debug, Deserialize)]
struct NargoPackage {
    name: String,
}

pub fn generate_proof(
    secret: &Secret,
    commitment: &Commitment,
    pda: &[u8; 32],
    program_id: &[u8; 32],
    action_hash: &[u8; 32],
    nullifier: &Nullifier,
    action_params: &[u8],
    nonce: u64,
    circuit_dir: &Path,
) -> Result<ProofResult, ProverError> {
    let circuit_name = read_circuit_name(circuit_dir)?;
    let target_dir = circuit_dir.join("target");
    let witness_name = "zaffre_witness";
    let prover_toml_path = circuit_dir.join("Prover.toml");

    let _prover_toml_guard = ProverTomlGuard::new(prover_toml_path);
    write_prover_toml(
        circuit_dir,
        secret,
        commitment,
        pda,
        program_id,
        action_hash,
        nullifier,
        action_params,
        nonce,
    )?;

    run_cmd(
        Command::new(tool_path("nargo"))
            .current_dir(circuit_dir)
            .arg("execute")
            .arg(witness_name),
    )?;

    let acir_path = target_dir.join(format!("{circuit_name}.json"));
    let witness_path = target_dir.join(format!("{witness_name}.gz"));
    let _witness_guard = TempFileGuard::new(witness_path.clone());
    if !acir_path.exists() {
        return Err(ProverError::MissingOutput(acir_path));
    }
    if !witness_path.exists() {
        return Err(ProverError::MissingOutput(witness_path));
    }
    let acir_rel = PathBuf::from("target").join(format!("{circuit_name}.json"));

    let ccs_path = target_dir.join(format!("{circuit_name}.ccs"));
    if !ccs_path.exists() {
        run_cmd(
            Command::new(tool_path("sunspot"))
                .current_dir(circuit_dir)
                .arg("compile")
                .arg(&acir_rel),
        )?;
    }
    if !ccs_path.exists() {
        return Err(ProverError::MissingOutput(ccs_path));
    }
    let ccs_rel = PathBuf::from("target").join(format!("{circuit_name}.ccs"));

    let pk_path = target_dir.join(format!("{circuit_name}.pk"));
    let vk_path = target_dir.join(format!("{circuit_name}.vk"));
    if !pk_path.exists() || !vk_path.exists() {
        run_cmd(
            Command::new(tool_path("sunspot"))
                .current_dir(circuit_dir)
                .arg("setup")
                .arg(&ccs_rel),
        )?;
    }
    if !pk_path.exists() {
        return Err(ProverError::MissingOutput(pk_path));
    }
    if !vk_path.exists() {
        return Err(ProverError::MissingOutput(vk_path));
    }
    let acir_file = PathBuf::from(format!("{circuit_name}.json"));
    let witness_file = PathBuf::from(format!("{witness_name}.gz"));
    let ccs_file = PathBuf::from(format!("{circuit_name}.ccs"));
    let pk_file = PathBuf::from(format!("{circuit_name}.pk"));

    run_cmd(
        Command::new(tool_path("sunspot"))
            .current_dir(&target_dir)
            .arg("prove")
            .arg(&acir_file)
            .arg(&witness_file)
            .arg(&ccs_file)
            .arg(&pk_file),
    )?;

    let proof_path = target_dir.join(format!("{circuit_name}.proof"));
    let public_witness_path = target_dir.join(format!("{circuit_name}.pw"));
    if !proof_path.exists() {
        return Err(ProverError::MissingOutput(proof_path));
    }
    if !public_witness_path.exists() {
        return Err(ProverError::MissingOutput(public_witness_path));
    }

    let proof = fs::read(&proof_path)?;
    let public_witness = fs::read(&public_witness_path)?;
    if public_witness.len() != PUBLIC_WITNESS_SIZE {
        return Err(ProverError::InvalidPublicWitness(public_witness.len()));
    }

    Ok(ProofResult {
        proof,
        public_witness,
        proof_path,
        public_witness_path,
    })
}

fn read_circuit_name(circuit_dir: &Path) -> Result<String, ProverError> {
    let manifest_path = circuit_dir.join("Nargo.toml");
    let manifest_str = fs::read_to_string(manifest_path)?;
    let manifest: NargoManifest = toml::from_str(&manifest_str)?;
    let name = manifest
        .package
        .and_then(|p| if p.name.trim().is_empty() { None } else { Some(p.name) })
        .ok_or(ProverError::MissingCircuitName)?;
    Ok(name)
}

fn write_prover_toml(
    circuit_dir: &Path,
    secret: &Secret,
    commitment: &Commitment,
    pda: &[u8; 32],
    program_id: &[u8; 32],
    action_hash: &[u8; 32],
    nullifier: &Nullifier,
    action_params: &[u8],
    nonce: u64,
) -> Result<(), ProverError> {
    let prover_path = circuit_dir.join("Prover.toml");
    let secret_field = bytes_to_field(secret.as_bytes());
    let secret_dec = field_to_decimal_string(&secret_field);
    if action_params.len() > ACTION_PARAMS_MAX_LEN {
        return Err(ProverError::ActionParamsTooLong(action_params.len()));
    }
    let mut action_params_padded = [0u8; ACTION_PARAMS_MAX_LEN];
    action_params_padded[..action_params.len()].copy_from_slice(action_params);
    let nonce_bytes = nonce.to_le_bytes();

    let content = format!(
        "secret = \"{}\"\ncommitment = {}\npda = {}\nprogram_id = {}\naction_hash = {}\nnullifier = {}\naction_params = {}\naction_params_len = {}\nnonce = {}\n",
        secret_dec,
        format_byte_array(commitment.as_bytes()),
        format_byte_array(pda),
        format_byte_array(program_id),
        format_byte_array(action_hash),
        format_byte_array(nullifier.as_bytes()),
        format_byte_array(&action_params_padded),
        action_params.len(),
        format_byte_array(&nonce_bytes),
    );
    fs::write(prover_path, content)?;
    Ok(())
}

fn field_to_decimal_string(field: &ark_bn254::Fr) -> String {
    let bytes = field.into_bigint().to_bytes_le();
    let n = BigUint::from_bytes_le(&bytes);
    n.to_str_radix(10)
}

fn format_byte_array(bytes: &[u8]) -> String {
    let items: Vec<String> = bytes.iter().map(|b| b.to_string()).collect();
    format!("[{}]", items.join(", "))
}

fn tool_path<S: AsRef<OsStr>>(name: S) -> PathBuf {
    let name_ref = name.as_ref();
    let env_key = format!(
        "{}_BIN",
        name_ref.to_string_lossy().to_uppercase()
    );
    if let Some(path) = std::env::var_os(env_key) {
        PathBuf::from(path)
    } else {
        PathBuf::from(name_ref)
    }
}

fn run_cmd(cmd: &mut Command) -> Result<(), ProverError> {
    let cmd_str = format!("{:?}", cmd);
    let output = cmd.output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            ProverError::MissingTool(cmd.get_program().to_string_lossy().to_string())
        } else {
            ProverError::Io(e)
        }
    })?;

    if !output.status.success() {
        return Err(ProverError::CommandFailed {
            cmd: cmd_str,
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        });
    }
    Ok(())
}

struct ProverTomlGuard {
    path: PathBuf,
    backup: Option<Vec<u8>>,
}

impl ProverTomlGuard {
    fn new(path: PathBuf) -> Self {
        let backup = fs::read(&path).ok();
        Self { path, backup }
    }
}

impl Drop for ProverTomlGuard {
    fn drop(&mut self) {
        if let Some(contents) = &self.backup {
            let _ = fs::write(&self.path, contents);
        } else {
            let _ = fs::remove_file(&self.path);
        }
    }
}

struct TempFileGuard {
    path: PathBuf,
}

impl TempFileGuard {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(name: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        let unique = format!(
            "zaffre_prover_test_{}_{}_{}",
            name,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        dir.push(unique);
        fs::create_dir_all(&dir).expect("failed to create temp dir");
        dir
    }

    #[test]
    fn test_action_params_len_too_long() {
        let dir = temp_dir("too_long");
        let secret = Secret::from_bytes([1u8; 32]);
        let commitment = Commitment::from_bytes([2u8; 32]);
        let nullifier = Nullifier::from_bytes([3u8; 32]);
        let pda = [4u8; 32];
        let program_id = [5u8; 32];
        let action_hash = [6u8; 32];
        let params = vec![0u8; ACTION_PARAMS_MAX_LEN + 1];

        let err = write_prover_toml(
            &dir,
            &secret,
            &commitment,
            &pda,
            &program_id,
            &action_hash,
            &nullifier,
            &params,
            1,
        )
        .expect_err("expected action params length error");

        match err {
            ProverError::ActionParamsTooLong(len) => {
                assert_eq!(len, ACTION_PARAMS_MAX_LEN + 1);
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn test_action_params_len_ok() {
        let dir = temp_dir("ok");
        let secret = Secret::from_bytes([1u8; 32]);
        let commitment = Commitment::from_bytes([2u8; 32]);
        let nullifier = Nullifier::from_bytes([3u8; 32]);
        let pda = [4u8; 32];
        let program_id = [5u8; 32];
        let action_hash = [6u8; 32];
        let params = vec![9u8; ACTION_PARAMS_MAX_LEN];

        write_prover_toml(
            &dir,
            &secret,
            &commitment,
            &pda,
            &program_id,
            &action_hash,
            &nullifier,
            &params,
            1,
        )
        .expect("expected write_prover_toml to succeed");

        let prover_path = dir.join("Prover.toml");
        assert!(prover_path.exists(), "Prover.toml not written");
    }
}
