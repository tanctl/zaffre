const wasm = require("../pkg/zaffre_sdk_js.js");
const web3 = require("@solana/web3.js");

const ZAFFRE_SEED = Buffer.from("zaffre");

function assertLen(name, bytes, expected) {
  const len = bytes.length ?? bytes.byteLength ?? 0;
  if (len !== expected) {
    throw new Error(`${name} must be ${expected} bytes (got ${len})`);
  }
}

function deriveZaffrePda(programIdBytes, commitmentBytes) {
  assertLen("programIdBytes", programIdBytes, 32);
  assertLen("commitmentBytes", commitmentBytes, 32);
  const programId = new web3.PublicKey(Buffer.from(programIdBytes));
  const [address, bump] = web3.PublicKey.findProgramAddressSync(
    [ZAFFRE_SEED, Buffer.from(commitmentBytes)],
    programId,
  );
  return { address: new Uint8Array(address.toBytes()), bump };
}

module.exports = {
  ...wasm,
  deriveZaffrePda,
};
