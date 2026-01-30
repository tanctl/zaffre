//! Allocation-free BN254 syscall wrappers for SBF.
//!
//! The Solana bump allocator never frees, so repeated `Vec` allocations will
//! eventually exhaust the 32 KiB heap. These helpers call the underlying
//! syscalls directly on-chain, while keeping the existing host behavior.

use solana_bn254::AltBn128Error;

#[cfg(target_os = "solana")]
use solana_bn254::prelude::{ALT_BN128_ADD, ALT_BN128_MUL, ALT_BN128_PAIRING};

#[cfg(not(target_os = "solana"))]
use solana_bn254::prelude::{
    alt_bn128_addition as alt_add, alt_bn128_multiplication as alt_mul, alt_bn128_pairing as alt_pair,
};

const ADD_INPUT_LEN: usize = 128;
const MUL_INPUT_LEN: usize = 96;
const ADD_OUTPUT_LEN: usize = 64;
const MUL_OUTPUT_LEN: usize = 64;
const PAIR_OUTPUT_LEN: usize = 32;

#[cfg(target_os = "solana")]
use solana_program::syscalls::sol_alt_bn128_group_op;

#[cfg(target_os = "solana")]
fn syscall(group_op: u64, input: &[u8], output: &mut [u8]) -> Result<(), AltBn128Error> {
    let res = unsafe {
        sol_alt_bn128_group_op(
            group_op,
            input.as_ptr(),
            input.len() as u64,
            output.as_mut_ptr(),
        )
    };
    if res == 0 {
        Ok(())
    } else {
        Err(AltBn128Error::from(res))
    }
}

/// Performs an alt_bn128 addition without heap allocation on SBF.
pub(crate) fn alt_bn128_addition(input: &[u8; ADD_INPUT_LEN]) -> Result<[u8; ADD_OUTPUT_LEN], AltBn128Error> {
    #[cfg(target_os = "solana")]
    {
        let mut out = [0u8; ADD_OUTPUT_LEN];
        syscall(ALT_BN128_ADD, input, &mut out)?;
        return Ok(out);
    }

    #[cfg(not(target_os = "solana"))]
    {
        let out = alt_add(input)?;
        let arr: [u8; ADD_OUTPUT_LEN] = out
            .as_slice()
            .try_into()
            .map_err(|_| AltBn128Error::SliceOutOfBounds)?;
        Ok(arr)
    }
}

/// Performs an alt_bn128 multiplication without heap allocation on SBF.
pub(crate) fn alt_bn128_multiplication(
    input: &[u8; MUL_INPUT_LEN],
) -> Result<[u8; MUL_OUTPUT_LEN], AltBn128Error> {
    #[cfg(target_os = "solana")]
    {
        let mut out = [0u8; MUL_OUTPUT_LEN];
        syscall(ALT_BN128_MUL, input, &mut out)?;
        return Ok(out);
    }

    #[cfg(not(target_os = "solana"))]
    {
        let out = alt_mul(input)?;
        let arr: [u8; MUL_OUTPUT_LEN] = out
            .as_slice()
            .try_into()
            .map_err(|_| AltBn128Error::SliceOutOfBounds)?;
        Ok(arr)
    }
}

/// Performs an alt_bn128 pairing without heap allocation on SBF.
pub(crate) fn alt_bn128_pairing(input: &[u8]) -> Result<[u8; PAIR_OUTPUT_LEN], AltBn128Error> {
    #[cfg(target_os = "solana")]
    {
        let mut out = [0u8; PAIR_OUTPUT_LEN];
        syscall(ALT_BN128_PAIRING, input, &mut out)?;
        return Ok(out);
    }

    #[cfg(not(target_os = "solana"))]
    {
        let out = alt_pair(input)?;
        let arr: [u8; PAIR_OUTPUT_LEN] = out
            .as_slice()
            .try_into()
            .map_err(|_| AltBn128Error::SliceOutOfBounds)?;
        Ok(arr)
    }
}
