import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { describe, it } from "node:test";

import * as anchor from "@coral-xyz/anchor";

const sdk = require("../../../zaffre-sdk-js/js/index.js");

describe("zaffre wasm + anchor compatibility", () => {
  it("derives PDA and action hash consistent with Anchor/web3", async () => {
    const prover = sdk.ZaffreProver.init();

    const secret: Uint8Array = prover.generateSecret();
    const commitment: Uint8Array = prover.computeCommitment(secret);

    const programId = new anchor.web3.PublicKey(new Uint8Array(32).fill(7));
    const [expectedPda, expectedBump] =
      anchor.web3.PublicKey.findProgramAddressSync(
        [Buffer.from("zaffre"), Buffer.from(commitment)],
        programId
      );
    const { address: derivedAddress, bump } = sdk.deriveZaffrePda(
      programId.toBytes(),
      commitment
    );
    assert.equal(Buffer.from(derivedAddress).toString("hex"), expectedPda.toBuffer().toString("hex"));
    assert.equal(bump, expectedBump);

    const actionDiscriminator = new Uint8Array(
      Buffer.from("setvalue").subarray(0, 8)
    );
    const actionParams = new Uint8Array([1, 2, 3, 4]);
    const nonce = 42n;

    const actionHash = prover.computeActionHash(
      programId.toBytes(),
      derivedAddress,
      actionDiscriminator,
      actionParams,
      nonce
    );
    const nullifier = prover.computeNullifier(
      secret,
      programId.toBytes(),
      actionHash
    );

    const nonceBytes = new Uint8Array(8);
    const view = new DataView(nonceBytes.buffer);
    view.setBigUint64(0, nonce, true);

    const expectedHash = createHash("sha256")
      .update(programId.toBytes())
      .update(Buffer.from(derivedAddress))
      .update(Buffer.from(actionDiscriminator))
      .update(Buffer.from(actionParams))
      .update(Buffer.from(nonceBytes))
      .digest();

    assert.equal(Buffer.from(actionHash).toString("hex"), expectedHash.toString("hex"));

    const publicWitness = prover.serializePublicInputs(
      commitment,
      derivedAddress,
      programId.toBytes(),
      actionHash,
      nullifier
    );

    const decodeRawInputs = (witness: Uint8Array): Uint8Array => {
      assert.equal(witness.length, 5132);
      const raw = new Uint8Array(160);
      let offset = 12;
      for (let i = 0; i < raw.length; i += 1) {
        raw[i] = witness[offset + 31];
        offset += 32;
      }
      return raw;
    };

    const rawInputs = decodeRawInputs(publicWitness);
    assert.equal(publicWitness.length, 5132);
    assert.equal(
      Buffer.from(rawInputs.slice(32, 64)).toString("hex"),
      Buffer.from(derivedAddress).toString("hex")
    );
    assert.equal(derivedAddress.length, 32);
    assert.equal(typeof expectedBump, "number");
  });
});
