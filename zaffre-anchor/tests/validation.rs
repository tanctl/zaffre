#[cfg(test)]
mod tests {
    use anchor_lang::prelude::*;
    use zaffre_anchor::*;

    #[test]
    fn test_valid_pda() {
        let program_id = Pubkey::new_unique();
        let commitment = [1u8; 32];
        let (expected, _) = Pubkey::find_program_address(
            &[ZAFFRE_SEED_PREFIX, &commitment],
            &program_id,
        );

        let (derived, _) = Pubkey::find_program_address(
            &[ZAFFRE_SEED_PREFIX, &commitment],
            &program_id,
        );
        assert_eq!(expected, derived);
    }
}
