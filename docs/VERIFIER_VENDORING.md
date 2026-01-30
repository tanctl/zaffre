# Verifier Vendoring (gnark-solana)

## Summary
We vendor a patched copy of `gnark-solana` under `vendor/gnark-solana` so the
Sunspot Groth16 verifier runs reliably on Solana 1.18 without SBF OOM.

## Why Vendor
- upstream verifier is a separate repo and not a workspace dependency
- default verifier allocates in tight loops, which OOMs on SBF because the bump allocator never frees
- vendoring makes the demo reproducible without extra repo setup

## What Breaks (Root Cause)
(github.com/reilabs/sunspot/issues/44)
On Solana, the program heap is a bump allocator (~32 KiB by default). It never frees. The default verifier repeatedly allocates `Vec` buffers inside verification loops, which accumulates until the heap is exhausted, yielding:
```
Error: memory allocation failed, out of memory
```
or an access violation in the heap section.

## Patch Strategy
Remove heap allocation from the hot path:
1) allocation‑free syscall wrappers  
2) no `Vec` in loops (use fixed stack buffers)

Host behavior stays the same, but on‑chain heap growth is eliminated.

## Changes live in
- `vendor/gnark-solana/crates/verifier-lib/src/syscalls.rs`
- `vendor/gnark-solana/crates/verifier-lib/src/verifier.rs`
- `vendor/gnark-solana/crates/verifier-lib/src/commitments.rs`
- `vendor/gnark-solana/crates/verifier-lib/src/lib.rs`

## Rebuild verifier (.so)
From the repo root:

```bash
export ZAFFRE_HOME=/path/to/zaffre
export SUNSPOT_HOME="$ZAFFRE_HOME/vendor/gnark-solana"

cd "$SUNSPOT_HOME/crates/verifier-bin"
PATH="$HOME/.local/share/solana/install/active_release/bin:$ZAFFRE_HOME/tools/solana/bin:$PATH" \
VK_PATH="$ZAFFRE_HOME/circuits/ownership/target/ownership.vk" \
TMPDIR="$ZAFFRE_HOME/tmp" \
CARGO_TARGET_DIR="$ZAFFRE_HOME/target-gnark" \
cargo build-sbf --tools-version v1.49 --sbf-out-dir "$ZAFFRE_HOME/circuits/ownership/target" -- --locked

cp "$ZAFFRE_HOME/circuits/ownership/target/verifier_bin.so" \
  "$ZAFFRE_HOME/circuits/ownership/target/ownership.so"
```

## Validation
After rebuilding, the Sunspot e2e test passes:

```bash
cd "$ZAFFRE_HOME/programs/example"
ZAFFRE_SUNSPOT_E2E=1 \
SUNSPOT_VERIFIER_SO="$ZAFFRE_HOME/circuits/ownership/target/ownership.so" \
ZAFFRE_PROOF_PATH="$ZAFFRE_HOME/circuits/ownership/target/ownership.proof" \
ZAFFRE_PUBLIC_WITNESS_PATH="$ZAFFRE_HOME/circuits/ownership/target/ownership.pw" \
ZAFFRE_VALUE=7 \
ZAFFRE_NONCE=1 \
cargo test sunspot_e2e -- --ignored
```
