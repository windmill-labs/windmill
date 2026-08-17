import { expect, test } from "bun:test";

import { looksLikeWorkspaceCiphertext } from "../src/commands/variable/variable.ts";

// =============================================================================
// looksLikeWorkspaceCiphertext drives whether single-file `variable push` treats
// a secret's value as already-encrypted (store verbatim) or as plaintext to be
// encrypted server-side. It must agree with the server guard
// (validate_already_encrypted_secret in windmill-store/src/variables.rs): a value
// is "ciphertext shaped" iff it is an external-backend marker, or standard base64
// decoding to a non-zero multiple of the AES block size (16 bytes).
// =============================================================================

test("treats workspace-ciphertext-shaped values as already-encrypted", () => {
  const ciphertextShaped = [
    "MpYeXnSBBF7dzI6K8J89xQ==", // real magic_crypt output: 16 bytes
    Buffer.alloc(16, 7).toString("base64"), // 16 bytes
    Buffer.alloc(32, 7).toString("base64"), // 32 bytes
    "$vault:f/x/cfg",
    "$aws_sm:f/x/cfg",
    "$azure_kv:f/x/cfg",
  ];
  for (const value of ciphertextShaped) {
    expect(looksLikeWorkspaceCiphertext(value)).toBe(true);
  }
});

test("treats hand-authored plaintext as NOT already-encrypted", () => {
  const plaintext = [
    "some: plaintext\n", // space, colon, newline
    "original-secret", // hyphen, not length % 4
    "hunter2",
    '{"a": 1}',
    "", // empty
    "dGVzdA==", // valid base64 but decodes to 4 bytes (not % 16)
    Buffer.alloc(17, 7).toString("base64"), // 17 bytes (not % 16)
    "$omething-plain", // starts with $ but is not a real backend marker
  ];
  for (const value of plaintext) {
    expect(looksLikeWorkspaceCiphertext(value)).toBe(false);
  }
});
