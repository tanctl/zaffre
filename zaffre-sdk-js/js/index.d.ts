export * from "../pkg/zaffre_sdk_js";

export function deriveZaffrePda(
  programIdBytes: Uint8Array,
  commitmentBytes: Uint8Array,
): { address: Uint8Array; bump: number };
