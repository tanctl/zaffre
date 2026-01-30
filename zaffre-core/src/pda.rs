use solana_program::pubkey::Pubkey;

use crate::types::{Commitment, Nullifier};

pub const ZAFFRE_SEED_PREFIX: &[u8] = b"zaffre";
pub const NULLIFIER_SEED_PREFIX: &[u8] = b"nullifier";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ZaffrePDA {
    pub address: Pubkey,
    pub bump: u8,
    pub commitment: Commitment,
}

pub fn derive_zaffre_pda(program_id: &Pubkey, commitment: &Commitment) -> ZaffrePDA {
    let (address, bump) = Pubkey::find_program_address(
        &[ZAFFRE_SEED_PREFIX, commitment.as_bytes()],
        program_id,
    );
    ZaffrePDA {
        address,
        bump,
        commitment: *commitment,
    }
}

pub fn derive_nullifier_pda(program_id: &Pubkey, nullifier: &Nullifier) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[NULLIFIER_SEED_PREFIX, nullifier.as_bytes()],
        program_id,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pda_determinism() {
        let commitment = Commitment::from_bytes([1u8; 32]);
        let program_id = Pubkey::new_unique();
        let pda1 = derive_zaffre_pda(&program_id, &commitment);
        let pda2 = derive_zaffre_pda(&program_id, &commitment);
        assert_eq!(pda1.address, pda2.address);
        assert_eq!(pda1.bump, pda2.bump);
    }

    #[test]
    fn test_different_commitments_different_pdas() {
        let c1 = Commitment::from_bytes([1u8; 32]);
        let c2 = Commitment::from_bytes([2u8; 32]);
        let program_id = Pubkey::new_unique();
        let pda1 = derive_zaffre_pda(&program_id, &c1);
        let pda2 = derive_zaffre_pda(&program_id, &c2);
        assert_ne!(pda1.address, pda2.address);
    }
}
