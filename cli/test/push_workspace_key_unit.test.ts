/**
 * Unit tests for pushWorkspaceKey in settings.ts.
 *
 * Covers WIN-2005: changing the encryption key in encryption_key.yaml and
 * pushing it must (by default) re-encrypt the remote secrets with the new key.
 *
 * Verifies that:
 * - an unchanged key is a no-op (no setWorkspaceEncryptionKey call)
 * - a changed key in non-interactive mode re-encrypts by default
 *   (skip_reencrypt = false), so secret plaintext values are preserved
 * - the --skip-reencrypt-on-key-change flag keeps the remote ciphertexts
 *   untouched (skip_reencrypt = true)
 * - WMILL_NO_REENCRYPT_ON_KEY_CHANGE=true does the same via env var
 */

import { expect, test, describe, beforeEach, afterEach, mock } from "bun:test";

// Track calls to mocked wmill functions
let remoteKey = "";
let setEncryptionKeyCalls: {
  workspace: string;
  requestBody: { new_key: string; skip_reencrypt?: boolean };
}[] = [];

// Mock the wmill module before importing settings.ts
mock.module("../gen/services.gen.ts", () => ({
  getWorkspaceEncryptionKey: async (_args: { workspace: string }) => ({
    key: remoteKey,
  }),
  setWorkspaceEncryptionKey: async (args: {
    workspace: string;
    requestBody: { new_key: string; skip_reencrypt?: boolean };
  }) => {
    setEncryptionKeyCalls.push(args);
  },
}));

import { pushWorkspaceKey } from "../src/core/settings.ts";

describe("pushWorkspaceKey", () => {
  const ws = "test-workspace";

  beforeEach(() => {
    remoteKey = "";
    setEncryptionKeyCalls = [];
    delete process.env.WMILL_NO_REENCRYPT_ON_KEY_CHANGE;
  });

  afterEach(() => {
    delete process.env.WMILL_NO_REENCRYPT_ON_KEY_CHANGE;
  });

  test("no-op when local key matches the remote key", async () => {
    remoteKey = "samekey";
    await pushWorkspaceKey(ws, "encryption_key", undefined, "samekey", {
      noninteractive: true,
    });
    expect(setEncryptionKeyCalls.length).toBe(0);
  });

  test("changed key re-encrypts by default in non-interactive mode", async () => {
    remoteKey = "oldkey";
    await pushWorkspaceKey(ws, "encryption_key", undefined, "newkey", {
      noninteractive: true,
    });
    expect(setEncryptionKeyCalls.length).toBe(1);
    expect(setEncryptionKeyCalls[0].requestBody.new_key).toBe("newkey");
    // skip_reencrypt false => backend re-encrypts existing secrets with new key
    expect(setEncryptionKeyCalls[0].requestBody.skip_reencrypt).toBe(false);
  });

  test("--skip-reencrypt-on-key-change skips re-encryption", async () => {
    remoteKey = "oldkey";
    await pushWorkspaceKey(ws, "encryption_key", undefined, "newkey", {
      noninteractive: true,
      skipReencrypt: true,
    });
    expect(setEncryptionKeyCalls.length).toBe(1);
    expect(setEncryptionKeyCalls[0].requestBody.new_key).toBe("newkey");
    expect(setEncryptionKeyCalls[0].requestBody.skip_reencrypt).toBe(true);
  });

  test("WMILL_NO_REENCRYPT_ON_KEY_CHANGE=true skips re-encryption non-interactively", async () => {
    remoteKey = "oldkey";
    process.env.WMILL_NO_REENCRYPT_ON_KEY_CHANGE = "true";
    await pushWorkspaceKey(ws, "encryption_key", undefined, "newkey", {
      noninteractive: true,
    });
    expect(setEncryptionKeyCalls.length).toBe(1);
    expect(setEncryptionKeyCalls[0].requestBody.new_key).toBe("newkey");
    expect(setEncryptionKeyCalls[0].requestBody.skip_reencrypt).toBe(true);
  });
});
