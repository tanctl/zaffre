//! anchor helpers for zaffre

pub mod sunspot;
pub mod validation;

pub use sunspot::{
    try_verify_ownership_proof, verify_ownership_proof, SunspotError, GROTH16_PROOF_SIZE,
    PUBLIC_WITNESS_SIZE,
};
pub use validation::{
    validate_nullifier_pda, validate_zaffre_pda, NullifierStateData, ZaffreError,
    NULLIFIER_SEED_PREFIX, NULLIFIER_STATE_SIZE, ZAFFRE_SEED_PREFIX,
};
