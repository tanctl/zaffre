use anchor_lang::prelude::*;
use solana_program::pubkey::Pubkey;

use zaffre_anchor::{verify_ownership_proof, NULLIFIER_SEED_PREFIX, ZAFFRE_SEED_PREFIX};
use zaffre_core::{compute_action_hash, serialize_public_witness};

declare_id!("HDGiZbLeKG5XqixDtAQb9dzuCiCTVP7Yg6En3ZyDHXM1");

pub const CONFIG_SEED: &[u8] = b"zaffre_config";

#[program]
pub mod example {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, verifier_program: Pubkey) -> Result<()> {
        ctx.accounts.config.authority = ctx.accounts.authority.key();
        ctx.accounts.config.verifier_program = verifier_program;
        Ok(())
    }

    pub fn prepare(
        _ctx: Context<Prepare>,
        commitment: [u8; 32],
        nullifier: [u8; 32],
    ) -> Result<()> {
        let _ = (commitment, nullifier);
        Ok(())
    }

    pub fn update_verifier(ctx: Context<UpdateVerifier>, verifier_program: Pubkey) -> Result<()> {
        ctx.accounts.config.verifier_program = verifier_program;
        Ok(())
    }

    pub fn set_value(
        ctx: Context<SetValue>,
        commitment: [u8; 32],
        nullifier: [u8; 32],
        proof: Vec<u8>,
        value: u64,
        nonce: u64,
    ) -> Result<()> {
        require!(
            ctx.accounts.nullifier_state.spent_at_slot == 0,
            ExampleError::NullifierAlreadySpent
        );

        let action_discriminator = *b"setvalue";
        let action_params = value.to_le_bytes();
        let action_hash = compute_action_hash(
            &ctx.program_id,
            &ctx.accounts.state.key(),
            &action_discriminator,
            &action_params,
            nonce,
        );

        let public_witness = serialize_public_witness(
            &commitment,
            &ctx.accounts.state.key(),
            &ctx.program_id,
            &action_hash,
            &nullifier,
        );

        verify_ownership_proof(&ctx.accounts.verifier_program, &proof, &public_witness)?;

        ctx.accounts.state.value = value;
        ctx.accounts.state.bump = ctx.bumps.state;
        ctx.accounts.state.commitment = commitment;
        ctx.accounts.nullifier_state.spent_at_slot = Clock::get()?.slot;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + Config::STATE_SIZE,
        seeds = [CONFIG_SEED],
        bump
    )]
    pub config: Account<'info, Config>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateVerifier<'info> {
    #[account(
        mut,
        seeds = [CONFIG_SEED],
        bump,
        has_one = authority
    )]
    pub config: Account<'info, Config>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(commitment: [u8; 32], nullifier: [u8; 32])]
pub struct Prepare<'info> {
    #[account(
        init_if_needed,
        payer = payer,
        space = 8 + DemoState::STATE_SIZE,
        seeds = [ZAFFRE_SEED_PREFIX, commitment.as_ref()],
        bump
    )]
    pub state: Account<'info, DemoState>,
    #[account(
        init_if_needed,
        payer = payer,
        space = 8 + NullifierState::STATE_SIZE,
        seeds = [NULLIFIER_SEED_PREFIX, nullifier.as_ref()],
        bump
    )]
    pub nullifier_state: Account<'info, NullifierState>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(commitment: [u8; 32], nullifier: [u8; 32])]
pub struct SetValue<'info> {
    #[account(
        seeds = [CONFIG_SEED],
        bump,
        has_one = verifier_program
    )]
    pub config: Account<'info, Config>,
    #[account(
        mut,
        seeds = [ZAFFRE_SEED_PREFIX, commitment.as_ref()],
        bump
    )]
    pub state: Account<'info, DemoState>,
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        mut,
        seeds = [NULLIFIER_SEED_PREFIX, nullifier.as_ref()],
        bump
    )]
    pub nullifier_state: Account<'info, NullifierState>,
    /// CHECK: sunspot verifier program is stateless
    pub verifier_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    pub clock: Sysvar<'info, Clock>,
}

#[account]
pub struct DemoState {
    pub value: u64,
    pub bump: u8,
    pub commitment: [u8; 32],
}

impl DemoState {
    pub const STATE_SIZE: usize = 8 + 1 + 32;
}

pub fn derive_state_pda(program_id: &Pubkey, commitment: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[ZAFFRE_SEED_PREFIX, commitment], program_id)
}

pub fn derive_config_pda(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[CONFIG_SEED], program_id)
}

#[account]
pub struct Config {
    pub authority: Pubkey,
    pub verifier_program: Pubkey,
}

impl Config {
    pub const STATE_SIZE: usize = 32 + 32;
}

#[account]
pub struct NullifierState {
    pub spent_at_slot: u64,
}

impl NullifierState {
    pub const STATE_SIZE: usize = 8;
}

#[error_code]
pub enum ExampleError {
    #[msg("Nullifier already spent")]
    NullifierAlreadySpent,
}
