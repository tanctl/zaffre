use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::instruction::Instruction;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;
use solana_program::sysvar;
use solana_program_test::{processor, ProgramTest};
use solana_sdk::signature::Signer;
use solana_sdk::transaction::Transaction;

use example::DemoState;
use zaffre_anchor::{GROTH16_PROOF_SIZE, NULLIFIER_SEED_PREFIX};

fn mock_verifier_process(
    _program_id: &Pubkey,
    _accounts: &[solana_program::account_info::AccountInfo],
    _data: &[u8],
) -> Result<(), ProgramError> {
    Ok(())
}

fn demo_processor<'a, 'b, 'c, 'd>(
    program_id: &'a Pubkey,
    accounts: &'b [AccountInfo<'c>],
    data: &'d [u8],
) -> ProgramResult {
    // programtest expects a higher-ranked function signature; anchor entry uses
    // a slice whose lifetime matches the account info lifetime and for tests this
    // transmute is safe because the accounts slice is only used for this call
    let accounts: &[AccountInfo<'c>] = unsafe { std::mem::transmute(accounts) };
    example::entry(program_id, accounts, data)
}

#[tokio::test]
async fn test_set_value_updates_state() {
    let verifier_program_id = Pubkey::new_unique();
    let mut program_test = ProgramTest::new(
        "example",
        example::id(),
        processor!(demo_processor),
    );
    program_test.add_program(
        "mock_verifier",
        verifier_program_id,
        processor!(mock_verifier_process),
    );

    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let commitment = [1u8; 32];
    let (state_pda, _) = example::derive_state_pda(&example::id(), &commitment);
    let nullifier = [9u8; 32];
    let (nullifier_pda, _) =
        Pubkey::find_program_address(&[NULLIFIER_SEED_PREFIX, &nullifier], &example::id());
    let (config_pda, _) = example::derive_config_pda(&example::id());

    let proof = vec![0u8; GROTH16_PROOF_SIZE];
    let value = 55u64;
    let nonce = 1u64;

    let init_ix = Instruction {
        program_id: example::id(),
        accounts: example::accounts::Initialize {
            config: config_pda,
            authority: payer.pubkey(),
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: example::instruction::Initialize {
            verifier_program: verifier_program_id,
        }
        .data(),
    };

    let mut init_tx = Transaction::new_with_payer(&[init_ix], Some(&payer.pubkey()));
    init_tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(init_tx).await.unwrap();

    let prepare_ix = Instruction {
        program_id: example::id(),
        accounts: example::accounts::Prepare {
            state: state_pda,
            nullifier_state: nullifier_pda,
            payer: payer.pubkey(),
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: example::instruction::Prepare {
            commitment,
            nullifier,
        }
        .data(),
    };

    let mut prepare_tx = Transaction::new_with_payer(&[prepare_ix], Some(&payer.pubkey()));
    prepare_tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(prepare_tx).await.unwrap();

    let accounts = example::accounts::SetValue {
        config: config_pda,
        state: state_pda,
        payer: payer.pubkey(),
        nullifier_state: nullifier_pda,
        verifier_program: verifier_program_id,
        system_program: system_program::ID,
        clock: sysvar::clock::ID,
    };

    let ix = Instruction {
        program_id: example::id(),
        accounts: accounts.to_account_metas(None),
        data: example::instruction::SetValue {
            commitment,
            nullifier,
            proof,
            value,
            nonce,
        }
        .data(),
    };

    let mut tx = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    let state_account = banks_client.get_account(state_pda).await.unwrap().unwrap();
    let mut data: &[u8] = &state_account.data;
    let state = DemoState::try_deserialize(&mut data).unwrap();

    assert_eq!(state.value, value);
    assert_eq!(state.commitment, commitment);
}

#[tokio::test]
async fn test_invalid_proof_length_fails() {
    let verifier_program_id = Pubkey::new_unique();
    let mut program_test = ProgramTest::new(
        "example",
        example::id(),
        processor!(demo_processor),
    );
    program_test.add_program(
        "mock_verifier",
        verifier_program_id,
        processor!(mock_verifier_process),
    );

    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let commitment = [2u8; 32];
    let (state_pda, _) = example::derive_state_pda(&example::id(), &commitment);
    let nullifier = [8u8; 32];
    let (nullifier_pda, _) =
        Pubkey::find_program_address(&[NULLIFIER_SEED_PREFIX, &nullifier], &example::id());
    let (config_pda, _) = example::derive_config_pda(&example::id());

    let proof = vec![0u8; 8];
    let value = 1u64;
    let nonce = 1u64;

    let init_ix = Instruction {
        program_id: example::id(),
        accounts: example::accounts::Initialize {
            config: config_pda,
            authority: payer.pubkey(),
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: example::instruction::Initialize {
            verifier_program: verifier_program_id,
        }
        .data(),
    };

    let mut init_tx = Transaction::new_with_payer(&[init_ix], Some(&payer.pubkey()));
    init_tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(init_tx).await.unwrap();

    let prepare_ix = Instruction {
        program_id: example::id(),
        accounts: example::accounts::Prepare {
            state: state_pda,
            nullifier_state: nullifier_pda,
            payer: payer.pubkey(),
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: example::instruction::Prepare {
            commitment,
            nullifier,
        }
        .data(),
    };

    let mut prepare_tx = Transaction::new_with_payer(&[prepare_ix], Some(&payer.pubkey()));
    prepare_tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(prepare_tx).await.unwrap();

    let accounts = example::accounts::SetValue {
        config: config_pda,
        state: state_pda,
        payer: payer.pubkey(),
        nullifier_state: nullifier_pda,
        verifier_program: verifier_program_id,
        system_program: system_program::ID,
        clock: sysvar::clock::ID,
    };

    let ix = Instruction {
        program_id: example::id(),
        accounts: accounts.to_account_metas(None),
        data: example::instruction::SetValue {
            commitment,
            nullifier,
            proof,
            value,
            nonce,
        }
        .data(),
    };

    let mut tx = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    assert!(banks_client.process_transaction(tx).await.is_err());
}

#[tokio::test]
async fn test_nullifier_replay_fails() {
    let verifier_program_id = Pubkey::new_unique();
    let mut program_test = ProgramTest::new(
        "example",
        example::id(),
        processor!(demo_processor),
    );
    program_test.add_program(
        "mock_verifier",
        verifier_program_id,
        processor!(mock_verifier_process),
    );

    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let commitment = [3u8; 32];
    let (state_pda, _) = example::derive_state_pda(&example::id(), &commitment);
    let nullifier = [7u8; 32];
    let (nullifier_pda, _) =
        Pubkey::find_program_address(&[NULLIFIER_SEED_PREFIX, &nullifier], &example::id());
    let (config_pda, _) = example::derive_config_pda(&example::id());

    let proof = vec![0u8; GROTH16_PROOF_SIZE];
    let value = 1u64;
    let nonce = 1u64;

    let init_ix = Instruction {
        program_id: example::id(),
        accounts: example::accounts::Initialize {
            config: config_pda,
            authority: payer.pubkey(),
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: example::instruction::Initialize {
            verifier_program: verifier_program_id,
        }
        .data(),
    };

    let mut init_tx = Transaction::new_with_payer(&[init_ix], Some(&payer.pubkey()));
    init_tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(init_tx).await.unwrap();

    let prepare_ix = Instruction {
        program_id: example::id(),
        accounts: example::accounts::Prepare {
            state: state_pda,
            nullifier_state: nullifier_pda,
            payer: payer.pubkey(),
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: example::instruction::Prepare {
            commitment,
            nullifier,
        }
        .data(),
    };

    let mut prepare_tx = Transaction::new_with_payer(&[prepare_ix], Some(&payer.pubkey()));
    prepare_tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(prepare_tx).await.unwrap();

    let accounts = example::accounts::SetValue {
        config: config_pda,
        state: state_pda,
        payer: payer.pubkey(),
        nullifier_state: nullifier_pda,
        verifier_program: verifier_program_id,
        system_program: system_program::ID,
        clock: sysvar::clock::ID,
    };

    let ix = Instruction {
        program_id: example::id(),
        accounts: accounts.to_account_metas(None),
        data: example::instruction::SetValue {
            commitment,
            nullifier,
            proof: proof.clone(),
            value,
            nonce,
        }
        .data(),
    };

    let mut tx = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
    tx.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(tx).await.unwrap();

    let replay_ix = Instruction {
        program_id: example::id(),
        accounts: accounts.to_account_metas(None),
        data: example::instruction::SetValue {
            commitment,
            nullifier,
            proof,
            value,
            nonce,
        }
        .data(),
    };

    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    let mut replay_tx = Transaction::new_with_payer(&[replay_ix], Some(&payer.pubkey()));
    replay_tx.sign(&[&payer], recent_blockhash);
    assert!(banks_client.process_transaction(replay_tx).await.is_err());
}
