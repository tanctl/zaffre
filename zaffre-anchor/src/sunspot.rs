//! sunspot verifier cpi helpers

use anchor_lang::prelude::*;
use solana_program::instruction::Instruction;
use solana_program::program::invoke;

#[error_code]
pub enum SunspotError {
    #[msg("Invalid proof length")]
    InvalidProofLength,
    #[msg("Invalid public witness length (expected 5132 bytes)")]
    InvalidPublicWitnessLength,
    #[msg("Verifier invocation failed")]
    VerifierInvocationFailed,
}

pub const GROTH16_PROOF_SIZE: usize = 388;

pub const PUBLIC_INPUT_BYTES: usize = 160;
pub const PUBLIC_INPUT_COUNT: usize = PUBLIC_INPUT_BYTES;
pub const PUBLIC_WITNESS_HEADER_LEN: usize = 12;
pub const PUBLIC_WITNESS_SIZE: usize = PUBLIC_WITNESS_HEADER_LEN + PUBLIC_INPUT_COUNT * 32;

pub fn verify_ownership_proof<'info>(
    verifier_program: &AccountInfo<'info>,
    proof: &[u8],
    public_witness: &[u8],
) -> Result<()> {
    require!(
        proof.len() == GROTH16_PROOF_SIZE,
        SunspotError::InvalidProofLength
    );
    require!(
        public_witness.len() == PUBLIC_WITNESS_SIZE,
        SunspotError::InvalidPublicWitnessLength
    );

    let mut instruction_data = Vec::with_capacity(proof.len() + PUBLIC_WITNESS_SIZE);
    instruction_data.extend_from_slice(proof);
    instruction_data.extend_from_slice(public_witness);

    let ix = Instruction {
        program_id: *verifier_program.key,
        accounts: vec![], // sunspot verifiers are stateless
        data: instruction_data,
    };

    invoke(&ix, &[]).map_err(|e| {
        msg!("Sunspot verification failed: {:?}", e);
        SunspotError::VerifierInvocationFailed
    })?;

    msg!("Zaffre ownership proof verified");
    Ok(())
}

pub fn try_verify_ownership_proof<'info>(
    verifier_program: &AccountInfo<'info>,
    proof: &[u8],
    public_witness: &[u8],
) -> bool {
    verify_ownership_proof(verifier_program, proof, public_witness).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(PUBLIC_INPUT_BYTES, 160);
        assert_eq!(PUBLIC_WITNESS_SIZE, 5132);
        assert_eq!(GROTH16_PROOF_SIZE, 388);
    }
}
