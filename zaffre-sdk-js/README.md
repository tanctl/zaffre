# Zaffre SDK (JS/WASM)
This crate provides wasm-bindgen bindings for Zaffre primitives:
- Secret generation
- Commitment / nullifier computation
- Action hash computation
- Public witness serialization (Gnark/Sunspot format)
- PDA derivation helper (via `@solana/web3.js` in a JS wrapper)

## Build
```bash
cd zaffre-sdk-js
wasm-pack build --target nodejs
```

## Usage (Node)
```javascript
const { ZaffreProver, deriveZaffrePda } = require("zaffre-sdk-js/js");
const prover = ZaffreProver.init();

const secret = prover.generateSecret();
const commitment = prover.computeCommitment(secret);

const programId = new Uint8Array(32); // your program id bytes
const { address } = deriveZaffrePda(programId, commitment);

const actionDiscriminator = new Uint8Array(8); // 8-byte discriminator
const actionParams = new Uint8Array([1, 2, 3]);
const nonce = 123n;

const actionHash = prover.computeActionHash(
  programId,
  address,
  actionDiscriminator,
  actionParams,
  nonce
);

const nullifier = prover.computeNullifier(secret, programId, actionHash);
const publicWitness = prover.serializePublicInputs(
  commitment,
  address,
  programId,
  actionHash,
  nullifier
);
```

## Usage (Bundler/Browser)
For ESM/bundlers, build with `wasm-pack build --target bundler` and import from the generated `pkg` bundle:
```javascript
import init, { ZaffreProver } from "zaffre-sdk-js";
import { deriveZaffrePda } from "zaffre-sdk-js/js";

await init();
const prover = ZaffreProver.init();
```

Note: `computeActionHash` expects a `u64`; in Node/Web, pass a `BigInt` (e.g., `123n`). `serializePublicInputs` returns the full Gnark public witness (5132 bytes for the ownership circuit), which you append to the proof bytes when calling the verifier.
For the bundled `circuits/ownership` circuit, `action_params_len` is capped at 32 bytes. The example program uses the 8-byte little-endian encoding of the `value` argument, and the nonce is a `u64`.