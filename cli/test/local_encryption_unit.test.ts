/**
 * Unit tests for local_encryption.ts encrypt/decrypt functions.
 * Tests round-trip encryption, different key lengths, and error handling.
 */

import { expect, test, describe } from "bun:test";
import { encrypt, decrypt } from "../src/utils/local_encryption.ts";

// =============================================================================
// encrypt / decrypt round-trip
// =============================================================================

describe("encrypt and decrypt", () => {
  test("round-trip with a simple message", async () => {
    const key = "my-secret-key";
    const message = "Hello, World!";
    const encrypted = await encrypt(message, key);
    const decrypted = await decrypt(encrypted, key);
    expect(decrypted).toBe(message);
  });

  test("round-trip with empty string", async () => {
    const key = "key";
    const encrypted = await encrypt("", key);
    const decrypted = await decrypt(encrypted, key);
    expect(decrypted).toBe("");
  });

  test("round-trip with long message", async () => {
    const key = "test-key-123";
    const message = "A".repeat(10000);
    const encrypted = await encrypt(message, key);
    const decrypted = await decrypt(encrypted, key);
    expect(decrypted).toBe(message);
  });

  test("round-trip with unicode characters", async () => {
    const key = "unicode-key";
    const message = "Hello 🌍 世界 مرحبا";
    const encrypted = await encrypt(message, key);
    const decrypted = await decrypt(encrypted, key);
    expect(decrypted).toBe(message);
  });

  test("round-trip with very short key", async () => {
    const key = "k";
    const message = "short key test";
    const encrypted = await encrypt(message, key);
    const decrypted = await decrypt(encrypted, key);
    expect(decrypted).toBe(message);
  });

  test("round-trip with very long key", async () => {
    const key = "x".repeat(1000);
    const message = "long key test";
    const encrypted = await encrypt(message, key);
    const decrypted = await decrypt(encrypted, key);
    expect(decrypted).toBe(message);
  });

  test("encrypted output is base64", async () => {
    const encrypted = await encrypt("test", "key");
    // base64 characters: A-Z, a-z, 0-9, +, /, =
    expect(encrypted).toMatch(/^[A-Za-z0-9+/=]+$/);
  });

  test("same message encrypted twice produces different ciphertexts (random IV)", async () => {
    const key = "determinism-test";
    const message = "same input";
    const enc1 = await encrypt(message, key);
    const enc2 = await encrypt(message, key);
    expect(enc1).not.toBe(enc2);
  });

  test("decrypting with wrong key throws", async () => {
    const encrypted = await encrypt("secret", "correct-key");
    await expect(decrypt(encrypted, "wrong-key")).rejects.toThrow();
  });

  test("decrypting corrupted ciphertext throws", async () => {
    await expect(decrypt("not-valid-ciphertext-at-all!!", "key")).rejects.toThrow();
  });

  test("round-trip with JSON content", async () => {
    const key = "json-key";
    const message = JSON.stringify({ license_key: "abc-123", secret: true });
    const encrypted = await encrypt(message, key);
    const decrypted = await decrypt(encrypted, key);
    expect(JSON.parse(decrypted)).toEqual({
      license_key: "abc-123",
      secret: true,
    });
  });
});
