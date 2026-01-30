# Zaffre Example Program
A minimal Anchor program that gates state mutation on a commitment-derived PDA and a Groth16 proof.

## What it demonstrates
- commitment-derived PDA
- on-chain action hash recomputation
- Groth16 proof verification via Sunspot verifier CPI
- nullifier replay protection (nullifier PDA is created once per action)
- verifier program pinning via config PDA

## Why tests/ lives here
This program is tiny, so its tests sit alongside it and exercise the full flow with program-test.

## Quickstart (local program-test)
```bash
cd programs/example
cargo test
```

## JS/TS SDK test
```bash
cd programs/example/ts
npm test
```

## On-chain devnet walkthrough

### 0) Environment
```bash
cd /path/to/zaffre
export ZAFFRE_HOME=$(pwd)
export SUNSPOT_HOME=${SUNSPOT_HOME:-"$ZAFFRE_HOME/vendor/gnark-solana"}
export ANCHOR_PROVIDER_URL=https://api.devnet.solana.com
export ANCHOR_WALLET=$HOME/.config/solana/id.json
solana config set --url https://api.devnet.solana.com
solana config set --keypair "$ANCHOR_WALLET"
solana airdrop 5
```

### 1) Deploy the example program
```bash
cd "$ZAFFRE_HOME"
mkdir -p target/deploy
# The repo includes a program keypair that matches `declare_id!`.
# If you generate a new keypair, update `declare_id!` and Anchor.toml to match.

cd "$ZAFFRE_HOME/programs/example"
PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH" \
  cargo build-sbf --tools-version v1.49

solana program deploy \
  --program-id "$ZAFFRE_HOME/target/deploy/example-keypair.json" \
  "$ZAFFRE_HOME/programs/example/target/deploy/example.so"

export PROGRAM_ID=$(solana address -k "$ZAFFRE_HOME/target/deploy/example-keypair.json")
```

### 2) Build wasm sdk + derive commitment + pda
```bash
cd "$ZAFFRE_HOME"
mkdir -p "$ZAFFRE_HOME/tmp" "$ZAFFRE_HOME/target-wasm" "$ZAFFRE_HOME/target-gnark"

cd "$ZAFFRE_HOME/zaffre-sdk-js"
TMPDIR="$ZAFFRE_HOME/tmp" CARGO_TARGET_DIR="$ZAFFRE_HOME/target-wasm" \
  wasm-pack build --target nodejs --release --no-opt

export SECRET_HEX=000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f
node <<'NODE'
const sdk = require("./js");
const { PublicKey } = require("@solana/web3.js");

const secret = Buffer.from(process.env.SECRET_HEX, "hex");
const prover = sdk.ZaffreProver.init();
const commitment = Buffer.from(prover.computeCommitment(secret));
const programId = new PublicKey(process.env.PROGRAM_ID);
const { address } = sdk.deriveZaffrePda(programId.toBytes(), commitment);

console.log("PROGRAM_ID_HEX=" + Buffer.from(programId.toBytes()).toString("hex"));
console.log("COMMITMENT_HEX=" + commitment.toString("hex"));
console.log("PDA_HEX=" + Buffer.from(address).toString("hex"));
NODE
```

### 3) Build circuit + verifier
```bash
cd "$ZAFFRE_HOME/circuits/ownership"
nargo compile
sunspot compile target/ownership.json
sunspot setup target/ownership.ccs

cd "$SUNSPOT_HOME/crates/verifier-bin"
PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH" \
VK_PATH="$ZAFFRE_HOME/circuits/ownership/target/ownership.vk" \
TMPDIR="$ZAFFRE_HOME/tmp" \
CARGO_TARGET_DIR="$ZAFFRE_HOME/target-gnark" \
cargo build-sbf --tools-version v1.49 --sbf-out-dir "$ZAFFRE_HOME/circuits/ownership/target" -- --locked

cp "$ZAFFRE_HOME/circuits/ownership/target/verifier_bin.so" \
  "$ZAFFRE_HOME/circuits/ownership/target/ownership.so"
```

### 4) Deploy the verifier program
```bash
cd "$ZAFFRE_HOME"
solana-keygen new -o circuits/ownership/target/ownership-verifier-keypair.json --force
solana program deploy \
  --program-id circuits/ownership/target/ownership-verifier-keypair.json \
  circuits/ownership/target/ownership.so
export VERIFIER_PROGRAM_ID=$(solana address -k circuits/ownership/target/ownership-verifier-keypair.json)
```

### 5) Initialize the config PDA
```bash
cd "$ZAFFRE_HOME/programs/example/ts"
node -e 'const anchor=require("@coral-xyz/anchor");\
const {PublicKey,SystemProgram}=anchor.web3; const provider=anchor.AnchorProvider.env();\
anchor.setProvider(provider); const idl={version:"0.1.0",name:"example",address:process.env.PROGRAM_ID,instructions:[{\
name:"initialize",discriminator:[175,175,109,31,13,152,155,237],accounts:[\
{name:"config",writable:true,signer:false},\
{name:"authority",writable:true,signer:true},\
{name:"systemProgram",writable:false,signer:false},\
],args:[{name:"verifierProgram",type:"publicKey"}]}]};\
const program=new anchor.Program(idl, provider); const programId=new PublicKey(process.env.PROGRAM_ID);\
const [config]=PublicKey.findProgramAddressSync([Buffer.from("zaffre_config")], programId);\
program.methods.initialize(new PublicKey(process.env.VERIFIER_PROGRAM_ID)).accounts({\
config, authority: provider.wallet.publicKey, systemProgram: SystemProgram.programId\
}).rpc().then(sig=>{console.log("tx",sig)}).catch(e=>{console.error(e); process.exit(1)});'
```

### 6) Generate proof
```bash
cd "$ZAFFRE_HOME/zaffre-prover"
NARGO_BIN="$(which nargo)" SUNSPOT_BIN="$(which sunspot)" \
  cargo run --bin zaffre_prove -- \
  "$ZAFFRE_HOME/circuits/ownership" \
  "$PROGRAM_ID_HEX" \
  "$PDA_HEX" \
  7 \
  1 \
  "$SECRET_HEX"

# capture the nullifier from stdout
# export NULLIFIER_HEX=...
# export PROOF_PATH=...

# Note: zaffre_prove reuses existing .pk/.vk to keep the verifier stable.
# Delete circuits/ownership/target/*.pk and *.vk to force a fresh setup.
```

### 7) Prepare PDAs (devnet)
```bash
cd "$ZAFFRE_HOME/programs/example/ts"
node -e 'const anchor=require("@coral-xyz/anchor");\
const {PublicKey,SystemProgram}=anchor.web3; const provider=anchor.AnchorProvider.env();\
anchor.setProvider(provider); const idl={version:"0.1.0",name:"example",address:process.env.PROGRAM_ID,instructions:[{\
name:"prepare",discriminator:[121,155,156,90,164,252,220,109],accounts:[\
{name:"state",writable:true,signer:false},\
{name:"nullifierState",writable:true,signer:false},\
{name:"payer",writable:true,signer:true},\
{name:"systemProgram",writable:false,signer:false},\
],args:[\
{name:"commitment",type:{array:["u8",32]}},\
{name:"nullifier",type:{array:["u8",32]}},\
]}]};\
const program=new anchor.Program(idl, provider); const programId=new PublicKey(process.env.PROGRAM_ID);\
const commitment=Buffer.from(process.env.COMMITMENT_HEX,"hex");\
const nullifier=Buffer.from(process.env.NULLIFIER_HEX,"hex");\
const [statePda]=PublicKey.findProgramAddressSync([Buffer.from("zaffre"), commitment], programId);\
const [nullifierPda]=PublicKey.findProgramAddressSync([Buffer.from("nullifier"), nullifier], programId);\
program.methods.prepare(Array.from(commitment), Array.from(nullifier)).accounts({\
state: statePda, nullifierState: nullifierPda, payer: provider.wallet.publicKey, systemProgram: SystemProgram.programId\
}).rpc().then(sig=>{console.log("prepare",sig)}).catch(e=>{console.error(e); process.exit(1)});'
```

### 8) Send the on-chain tx (devnet)
```bash
cd "$ZAFFRE_HOME/programs/example/ts"
export ZAFFRE_VALUE=7
export ZAFFRE_NONCE=1
export PROOF_PATH="$ZAFFRE_HOME/circuits/ownership/target/ownership.proof"
export NULLIFIER_HEX=... # from step 6 output
node -e 'const fs=require("fs"); const anchor=require("@coral-xyz/anchor");\
const {PublicKey,SystemProgram,ComputeBudgetProgram}=anchor.web3; const provider=anchor.AnchorProvider.env();\
anchor.setProvider(provider); const idl={version:"0.1.0",name:"example",address:process.env.PROGRAM_ID,instructions:[{\
name:"setValue",discriminator:[253,214,48,201,100,201,227,219],accounts:[\
{name:"config",writable:false,signer:false},\
{name:"state",writable:true,signer:false},\
{name:"payer",writable:true,signer:true},\
{name:"nullifierState",writable:true,signer:false},\
{name:"verifierProgram",writable:false,signer:false},\
{name:"systemProgram",writable:false,signer:false},\
{name:"clock",writable:false,signer:false},\
],args:[\
{name:"commitment",type:{array:["u8",32]}},\
{name:"nullifier",type:{array:["u8",32]}},\
{name:"proof",type:"bytes"},\
{name:"value",type:"u64"},\
{name:"nonce",type:"u64"},\
]}]};\
const program=new anchor.Program(idl, provider); const programId=new PublicKey(process.env.PROGRAM_ID);\
const commitment=Buffer.from(process.env.COMMITMENT_HEX,"hex");\
const nullifier=Buffer.from(process.env.NULLIFIER_HEX,"hex");\
const [config]=PublicKey.findProgramAddressSync([Buffer.from("zaffre_config")], programId);\
const [statePda]=PublicKey.findProgramAddressSync([Buffer.from("zaffre"), commitment], programId);\
const [nullifierPda]=PublicKey.findProgramAddressSync([Buffer.from("nullifier"), nullifier], programId);\
const proof=fs.readFileSync(process.env.PROOF_PATH);\
const value=new anchor.BN(process.env.ZAFFRE_VALUE); const nonce=new anchor.BN(process.env.ZAFFRE_NONCE);\
const cuIx=ComputeBudgetProgram.setComputeUnitLimit({units: 1400000});\
program.methods.setValue(Array.from(commitment), Array.from(nullifier), proof, value, nonce).preInstructions([cuIx]).accounts({\
config, state: statePda, payer: provider.wallet.publicKey, nullifierState: nullifierPda, verifierProgram: new PublicKey(process.env.VERIFIER_PROGRAM_ID),\
systemProgram: SystemProgram.programId, clock: anchor.web3.SYSVAR_CLOCK_PUBKEY\
}).rpc().then(sig=>{console.log("tx",sig)}).catch(e=>{console.error(e); process.exit(1)});'
```
