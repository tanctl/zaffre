use std::path::PathBuf;

use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::instruction::Instruction;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;
use solana_program::sysvar;
use solana_program_test::{processor, ProgramTest};
use solana_sdk::signature::Signer;
use solana_sdk::transaction::Transaction;

use zaffre_core::{
    compute_action_hash, extract_public_inputs, PUBLIC_INPUT_BYTES, PUBLIC_WITNESS_SIZE,
};
use example::DemoState;
use zaffre_anchor::NULLIFIER_SEED_PREFIX;

fn read_public_inputs(path: &PathBuf) -> [u8; PUBLIC_INPUT_BYTES] {
    let bytes = std::fs::read(path).expect("failed to read public witness");
    assert_eq!(
        bytes.len(),
        PUBLIC_WITNESS_SIZE,
        "public witness must be {PUBLIC_WITNESS_SIZE} bytes",
    );
    extract_public_inputs(&bytes).expect("public witness header/shape invalid")
}

fn demo_processor<'a, 'b, 'c, 'd>(
    program_id: &'a Pubkey,
    accounts: &'b [AccountInfo<'c>],
    data: &'d [u8],
) -> ProgramResult {
    let accounts: &[AccountInfo<'c>] = unsafe { std::mem::transmute(accounts) };
    example::entry(program_id, accounts, data)
}

#[tokio::test]
#[ignore]
async fn test_set_value_with_real_sunspot_verifier() {
    if std::env::var("ZAFFRE_SUNSPOT_E2E").is_err() {
        eprintln!("Skipping: set ZAFFRE_SUNSPOT_E2E=1 to run Sunspot e2e test.");
        return;
    }

    let verifier_so = PathBuf::from(
        std::env::var("SUNSPOT_VERIFIER_SO")
            .expect("SUNSPOT_VERIFIER_SO must point to verifier .so"),
    );
    let proof_path = PathBuf::from(
        std::env::var("ZAFFRE_PROOF_PATH")
            .expect("ZAFFRE_PROOF_PATH must point to proof file"),
    );
    let witness_path = PathBuf::from(
        std::env::var("ZAFFRE_PUBLIC_WITNESS_PATH")
            .expect("ZAFFRE_PUBLIC_WITNESS_PATH must point to public witness file"),
    );

    let value: u64 = std::env::var("ZAFFRE_VALUE")
        .expect("ZAFFRE_VALUE must be set")
        .parse()
        .expect("ZAFFRE_VALUE must be u64");
    let nonce: u64 = std::env::var("ZAFFRE_NONCE")
        .expect("ZAFFRE_NONCE must be set")
        .parse()
        .expect("ZAFFRE_NONCE must be u64");

    let public_inputs = read_public_inputs(&witness_path);
    let commitment = public_inputs[0..32].try_into().unwrap();
    let pda_bytes: [u8; 32] = public_inputs[32..64].try_into().unwrap();
    let program_id_bytes: [u8; 32] = public_inputs[64..96].try_into().unwrap();
    let action_hash: [u8; 32] = public_inputs[96..128].try_into().unwrap();
    let nullifier: [u8; 32] = public_inputs[128..PUBLIC_INPUT_BYTES].try_into().unwrap();

    let program_id = example::id();
    assert_eq!(
        program_id_bytes,
        program_id.to_bytes(),
        "public witness program_id does not match example program id"
    );

    let expected_pda = example::derive_state_pda(&program_id, &commitment).0;
    assert_eq!(
        pda_bytes,
        expected_pda.to_bytes(),
        "public witness PDA does not match commitment-derived PDA"
    );

    let action_discriminator = *b"setvalue";
    let expected_action_hash = compute_action_hash(
        &program_id,
        &expected_pda,
        &action_discriminator,
        &value.to_le_bytes(),
        nonce,
    );
    assert_eq!(
        action_hash, expected_action_hash,
        "public witness action_hash does not match value/nonce"
    );

    let verifier_program_id = Pubkey::new_unique();
    let program_name = verifier_so
        .file_stem()
        .expect("invalid verifier .so name")
        .to_string_lossy()
        .to_string();
    let verifier_dir = verifier_so
        .parent()
        .expect("verifier .so must have a parent dir");

    let mut program_test = ProgramTest::new(
        "example",
        program_id,
        processor!(demo_processor),
    );
    std::env::set_var("BPF_OUT_DIR", verifier_dir);
    program_test.prefer_bpf(true);
    program_test.set_compute_max_units(1_400_000);
    program_test.add_program(&program_name, verifier_program_id, None);

    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let proof = std::fs::read(&proof_path).expect("failed to read proof");
    let (config_pda, _) = example::derive_config_pda(&program_id);

    let init_ix = Instruction {
        program_id,
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
        program_id,
        accounts: example::accounts::Prepare {
            state: expected_pda,
            nullifier_state: Pubkey::find_program_address(
                &[NULLIFIER_SEED_PREFIX, &nullifier],
                &program_id,
            )
            .0,
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
        state: expected_pda,
        payer: payer.pubkey(),
        nullifier_state: Pubkey::find_program_address(
            &[NULLIFIER_SEED_PREFIX, &nullifier],
            &program_id,
        )
        .0,
        verifier_program: verifier_program_id,
        system_program: system_program::ID,
        clock: sysvar::clock::ID,
    };

    let ix = Instruction {
        program_id,
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

    let state_account = banks_client
        .get_account(expected_pda)
        .await
        .unwrap()
        .unwrap();
    let mut data: &[u8] = &state_account.data;
    let state = DemoState::try_deserialize(&mut data).unwrap();

    assert_eq!(state.value, value);
    assert_eq!(state.commitment, commitment);
}
