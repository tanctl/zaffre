# Zaffre - Unlinkable Program-Derived Addresses for Solana

## Overview
Zaffre is a privacy‑tooling SDK for Solana that lets apps store state at **commitment‑derived PDAs** and verify **zero‑knowledge proofs of ownership** on‑chain. It makes it easy to:
- derive PDAs from a commitment instead of a public key
- recompute an action hash on‑chain
- verify a Groth16 proof (via Sunspot) that a secret exists for the commitment used to derive that PDA
The result: **state can be unlinkable when users rotate secrets**, while still being verifiable by on‑chain programs.

## Background (Why Zaffre)
On Solana, PDAs are deterministic and often derived from stable public keys or static seeds. That makes state graphs **trivially linkable**. Zaffre swaps the seed to a commitment (hash of a secret). Users can rotate secrets to get fresh, unlinkable PDAs without changing the program logic.

## PDA Enumeration: Why Zaffre Exists
Standard Anchor PDAs can be enumerated from public seeds:
```ts
const [pda] = findProgramAddressSync(
  [Buffer.from("profile"), userPubkey.toBuffer()],
  programId
);
```
Anyone can derive and inspect this account.

With Zaffre, the seed is a commitment:
```ts
const [pda] = findProgramAddressSync(
  [Buffer.from("zaffre"), commitment],
  programId
);
```
Without the secret, enumeration is computationally infeasible.

## Core idea
- user keeps a 32-byte secret
- commitment = Poseidon(secret)
- PDA is derived from ("zaffre", commitment)
- user proves knowledge of the secret and action hash off-chain
- program verifies the Groth16 proof via a Sunspot verifier CPI

## Components
- `zaffre-core` commit and action hash utilities, witness parsing
- `zaffre-anchor` on-chain helpers for PDA validation and proof verification
- `zaffre-prover` CLI that shells out to nargo + sunspot for proof generation
- `zaffre-sdk-js` wasm/JS bindings for commitments, PDAs, and witness encoding
- `circuits/ownership` noir circuit + proving artifacts
- `programs/example` minimal Anchor program with tests

## Example program
See `programs/example/README.md` for end-to-end on-chain devnet walkthrough

## How it works
1) User generates a 32-byte secret `s` and computes a commitment `c = Poseidon(s)` (using Noir's Poseidon parameters).
2) The PDA is derived as `PDA = find_program_address([b"zaffre", c], program_id)`.
3) The client computes an action hash `h = SHA256(program_id || pda || discriminator || params || nonce_le_u64)` and a nullifier `n = Poseidon(secret, SHA256(program_id), h)`.
4) Off-chain, the prover writes `Prover.toml` (including the action params + nonce as private inputs), runs `nargo execute` to build the witness, then runs `sunspot compile / setup / prove` to emit:
   - `*.proof` (Groth16 proof)
   - `*.pw` (gnark public witness format)
5) On-chain, the program recomputes `h`, validates the PDA seed prefix and commitment, and extracts public inputs from the witness bytes.
6) The program CPI-calls the Sunspot verifier with `(proof || public_witness)` against the circuit's embedded verifying key.
7) If the verifier accepts and the public inputs match `(c, pda, program_id, h, n)`, and the circuit recomputes `h` from the action params + nonce and `n` from `(secret, program_id, h)`, the state mutation is applied. The example program pins the verifier program id via a config PDA.

## Prereqs
- rust toolchain
- solana cli 1.18.26
- anchor 0.32.1
- noir/nargo 1.0.0-beta.18
- sunspot
- wasm-pack
- node 18+

### Install
```bash
# solana cli (1.18.26)
sh -c "$(curl -sSfL https://release.solana.com/v1.18.26/install)"

# anchor (avm)
cargo install --git https://github.com/coral-xyz/anchor avm --locked
avm install 0.32.1
avm use 0.32.1

# noir + nargo (1.0.0-beta.18)
curl -L https://raw.githubusercontent.com/noir-lang/noirup/main/install | bash
"$HOME/.nargo/bin/noirup" -v 1.0.0-beta.18

# sunspot
cargo install --git https://github.com/reilabs/sunspot --locked

# wasm-pack
cargo install wasm-pack --locked
```

## Build + Test
### Rust crates
```bash
cargo test -p zaffre-core
cargo test -p zaffre-anchor
```

### Prover CLI
```bash
cd zaffre-prover
cargo test
```

### JS/WASM SDK
```bash
cd zaffre-sdk-js
wasm-pack build --target nodejs --release --no-opt
```

### Example program (local program-test)
```bash
cd programs/example
cargo test
```

### Example TS SDK test
```bash
cd programs/example/ts
npm test
```

### Sunspot e2e test
```bash
export ZAFFRE_SUNSPOT_E2E=1
export SUNSPOT_VERIFIER_SO=/absolute/path/to/ownership.so
export ZAFFRE_PROOF_PATH=/absolute/path/to/ownership.proof
export ZAFFRE_PUBLIC_WITNESS_PATH=/absolute/path/to/ownership.pw
export ZAFFRE_VALUE=7
export ZAFFRE_NONCE=1
cd programs/example
cargo test sunspot_e2e -- --ignored
```

## Composability
Zaffre only affects PDA derivation and proof verification. Composes with any privacy stack (relayers, encrypted state, mixers, private payments) because Anchor accounts and CPI flows remain standard.
```ts
const { address: zaffrePda } = deriveZaffrePda(programIdBytes, commitment);
await program.methods
  .prepare(Array.from(commitment), Array.from(nullifier))
  .accounts({ state: zaffrePda, nullifierState: nullifierPda, payer, systemProgram })
  .rpc();
await program.methods
  .setValue(Array.from(commitment), Array.from(nullifier), proof, value, nonce)
  .accounts({ config, state: zaffrePda, nullifierState: nullifierPda, payer, verifierProgram, systemProgram, clock })
  .rpc();
```

## Security model
**Privacy Guarantees:**
- Address unlinkability for commitment‑derived PDAs (without the secret, PDAs are not enumerable).
- Proofs are bound to `(program_id, pda, action_discriminator, action_params, nonce)` via `action_hash`.
- Replay protection is application‑level when a nullifier PDA is enforced.

**Assumptions:**
- Secrets are generated securely and not reused across unlinkability domains.
- The intended circuit + verifier program are used and pinned on-chain.
- Clients compute `action_hash`/`nullifier` consistently with `action_params_len`.

**Out of scope:**
- Network‑layer anonymity, mempool privacy, or fee‑payer privacy.
- Hiding state addresses once revealed on‑chain, or transaction metadata (payer, program id, accounts, logs).
- Protection against compromised client devices.
 
**Trusted setup / verifier integrity:**
- The verifier embeds the Groth16 verification key at build time.
- Production deployments should use an MPC trusted setup and pin the verifier program id.

## Limitations
- Unlinkability depends on secret rotation and never reusing commitments
- Proof generation is off-chain and requires Noir/Sunspot tooling
- Groth16 verification is compute-heavy and needs high CU budgets
- The bundled ownership circuit supports action params up to 32 bytes

## Docs
- `docs/VERIFIER_VENDORING.md`
