/**
 * Lock Cache Tests
 *
 * Tests the in-memory lock cache used when fetching lockfiles for scripts with
 * raw_workspace_dependencies.
 *
 * Part 1: Unit tests for cache key computation.
 * Part 2: Behavioral tests comparing old logic (no cache, always fetches)
 *         vs new logic (caches by key, skips duplicate fetches).
 */

import {
  assertEquals,
  assertNotEquals,
} from "https://deno.land/std@0.224.0/assert/mod.ts";
import { encodeHex } from "https://deno.land/std@0.224.0/encoding/hex.ts";

// ---------------------------------------------------------------------------
// Mirrors computeLockCacheKey from src/utils/metadata.ts so we can test the
// algorithm without pulling in the full (unresolvable-in-tests) module graph.
// The key is (language, rawWorkspaceDependencies) — scriptContent is NOT part
// of the key because all scripts sharing the same raw deps get the same lock.
// ---------------------------------------------------------------------------
async function computeLockCacheKey(
  language: string,
  rawWorkspaceDependencies: Record<string, string>,
): Promise<string> {
  const sortedDepsKeys = Object.keys(rawWorkspaceDependencies).sort();
  const depsStr = sortedDepsKeys
    .map((k) => `${k}=${rawWorkspaceDependencies[k]}`)
    .join(";");
  const content = `${language}|${depsStr}`;
  const buf = new TextEncoder().encode(content);
  return encodeHex(await crypto.subtle.digest("SHA-256", buf));
}

// ---------------------------------------------------------------------------
// Helpers that mirror the two fetch strategies (old / new).
// `remoteFn` stands in for the real network call and lets us count invocations.
// ---------------------------------------------------------------------------

interface ScriptInput {
  scriptContent: string;
  language: string;
  remotePath: string;
  rawWorkspaceDependencies: Record<string, string>;
}

/** Old logic: always calls the remote for every script. */
async function fetchScriptLockOld(
  input: ScriptInput,
  remoteFn: (input: ScriptInput) => Promise<string>,
): Promise<string> {
  return await remoteFn(input);
}

/** New logic: only caches when raw_workspace_dependencies are non-empty. */
async function fetchScriptLockNew(
  input: ScriptInput,
  remoteFn: (input: ScriptInput) => Promise<string>,
  cache: Map<string, string>,
): Promise<string> {
  const hasRawDeps = Object.keys(input.rawWorkspaceDependencies).length > 0;
  const cacheKey = hasRawDeps
    ? await computeLockCacheKey(
        input.language,
        input.rawWorkspaceDependencies,
      )
    : undefined;
  if (cacheKey && cache.has(cacheKey)) {
    return cache.get(cacheKey)!;
  }
  const lock = await remoteFn(input);
  if (cacheKey) {
    cache.set(cacheKey, lock);
  }
  return lock;
}

// =============================================================================
// Part 1 — Cache key computation
// =============================================================================

Deno.test("same inputs produce same key", async () => {
  const a = await computeLockCacheKey("python3", {
    "dependencies/requirements.in": "requests==2.31.0",
  });
  const b = await computeLockCacheKey("python3", {
    "dependencies/requirements.in": "requests==2.31.0",
  });
  assertEquals(a, b);
});

Deno.test("empty deps produce same key", async () => {
  assertEquals(
    await computeLockCacheKey("python3", {}),
    await computeLockCacheKey("python3", {}),
  );
});

Deno.test("dep key order does not matter", async () => {
  const a = await computeLockCacheKey("python3", {
    "dependencies/requirements.in": "requests",
    "dependencies/base/requirements.in": "flask",
  });
  const b = await computeLockCacheKey("python3", {
    "dependencies/base/requirements.in": "flask",
    "dependencies/requirements.in": "requests",
  });
  assertEquals(a, b);
});

Deno.test("different dep content → different key", async () => {
  assertNotEquals(
    await computeLockCacheKey("python3", { d: "a" }),
    await computeLockCacheKey("python3", { d: "b" }),
  );
});

Deno.test("different script content, same language+deps → same key", async () => {
  assertEquals(
    await computeLockCacheKey("python3", { d: "v" }),
    await computeLockCacheKey("python3", { d: "v" }),
  );
});

Deno.test("different language → different key", async () => {
  assertNotEquals(
    await computeLockCacheKey("bun", { d: "v" }),
    await computeLockCacheKey("deno", { d: "v" }),
  );
});

Deno.test("extra dep entry → different key", async () => {
  assertNotEquals(
    await computeLockCacheKey("python3", { a: "1" }),
    await computeLockCacheKey("python3", { a: "1", b: "2" }),
  );
});

// =============================================================================
// Part 2 — Multi-script fetch behavior: old logic vs new logic
// =============================================================================

function makeRemoteFn(): {
  remoteFn: (input: ScriptInput) => Promise<string>;
  callCount: () => number;
  calledWith: () => ScriptInput[];
} {
  const calls: ScriptInput[] = [];
  return {
    remoteFn: async (input: ScriptInput) => {
      calls.push(input);
      const depsStr = Object.entries(input.rawWorkspaceDependencies).sort().map(([k,v]) => `${k}=${v}`).join(",");
      return `lock for ${input.language}:${input.scriptContent}:${depsStr}`;
    },
    callCount: () => calls.length,
    calledWith: () => calls,
  };
}

// -- Two scripts, same content + language + deps ----------------------------

Deno.test("old logic: two identical scripts → 2 remote calls", async () => {
  const { remoteFn, callCount } = makeRemoteFn();
  const deps = { "dependencies/requirements.in": "requests==2.31.0" };

  const scripts: ScriptInput[] = [
    { scriptContent: "print(1)", language: "python3", remotePath: "u/a/s1", rawWorkspaceDependencies: deps },
    { scriptContent: "print(1)", language: "python3", remotePath: "u/a/s2", rawWorkspaceDependencies: deps },
  ];

  const results: string[] = [];
  for (const s of scripts) {
    results.push(await fetchScriptLockOld(s, remoteFn));
  }

  assertEquals(callCount(), 2);
  assertEquals(results[0], results[1]);
});

Deno.test("new logic: two identical scripts → 1 remote call", async () => {
  const { remoteFn, callCount } = makeRemoteFn();
  const cache = new Map<string, string>();
  const deps = { "dependencies/requirements.in": "requests==2.31.0" };

  const scripts: ScriptInput[] = [
    { scriptContent: "print(1)", language: "python3", remotePath: "u/a/s1", rawWorkspaceDependencies: deps },
    { scriptContent: "print(1)", language: "python3", remotePath: "u/a/s2", rawWorkspaceDependencies: deps },
  ];

  const results: string[] = [];
  for (const s of scripts) {
    results.push(await fetchScriptLockNew(s, remoteFn, cache));
  }

  assertEquals(callCount(), 1);
  assertEquals(results[0], results[1]);
});

// -- Two scripts, same deps but different content ---------------------------

Deno.test("old logic: two different scripts same deps → 2 remote calls", async () => {
  const { remoteFn, callCount } = makeRemoteFn();
  const deps = { "dependencies/requirements.in": "requests==2.31.0" };

  const scripts: ScriptInput[] = [
    { scriptContent: "print(1)", language: "python3", remotePath: "u/a/s1", rawWorkspaceDependencies: deps },
    { scriptContent: "print(2)", language: "python3", remotePath: "u/a/s2", rawWorkspaceDependencies: deps },
  ];

  for (const s of scripts) await fetchScriptLockOld(s, remoteFn);
  assertEquals(callCount(), 2);
});

Deno.test("new logic: two different scripts same deps → 1 remote call (cache shared)", async () => {
  const { remoteFn, callCount } = makeRemoteFn();
  const cache = new Map<string, string>();
  const deps = { "dependencies/requirements.in": "requests==2.31.0" };

  const scripts: ScriptInput[] = [
    { scriptContent: "print(1)", language: "python3", remotePath: "u/a/s1", rawWorkspaceDependencies: deps },
    { scriptContent: "print(2)", language: "python3", remotePath: "u/a/s2", rawWorkspaceDependencies: deps },
  ];

  const results: string[] = [];
  for (const s of scripts) results.push(await fetchScriptLockNew(s, remoteFn, cache));
  assertEquals(callCount(), 1);
  // Both get the same cached lock since language+deps are the same
  assertEquals(results[0], results[1]);
});

// -- Two scripts, same content but different deps ---------------------------

Deno.test("old logic: same content different deps → 2 remote calls", async () => {
  const { remoteFn, callCount } = makeRemoteFn();

  const scripts: ScriptInput[] = [
    { scriptContent: "print(1)", language: "python3", remotePath: "u/a/s1",
      rawWorkspaceDependencies: { "dependencies/requirements.in": "requests==2.31.0" } },
    { scriptContent: "print(1)", language: "python3", remotePath: "u/a/s2",
      rawWorkspaceDependencies: { "dependencies/requirements.in": "requests==2.32.0" } },
  ];

  for (const s of scripts) await fetchScriptLockOld(s, remoteFn);
  assertEquals(callCount(), 2);
});

Deno.test("new logic: same content different deps → 2 remote calls (no false cache hit)", async () => {
  const { remoteFn, callCount } = makeRemoteFn();
  const cache = new Map<string, string>();

  const scripts: ScriptInput[] = [
    { scriptContent: "print(1)", language: "python3", remotePath: "u/a/s1",
      rawWorkspaceDependencies: { "dependencies/requirements.in": "requests==2.31.0" } },
    { scriptContent: "print(1)", language: "python3", remotePath: "u/a/s2",
      rawWorkspaceDependencies: { "dependencies/requirements.in": "requests==2.32.0" } },
  ];

  const results: string[] = [];
  for (const s of scripts) results.push(await fetchScriptLockNew(s, remoteFn, cache));
  assertEquals(callCount(), 2);
  // Results differ because deps are different
  assertNotEquals(results[0], results[1]);
});

// -- Many scripts, mix of content but same language + deps ------------------

Deno.test("old logic: 5 scripts same deps → 5 remote calls", async () => {
  const { remoteFn, callCount } = makeRemoteFn();
  const deps = { "dependencies/requirements.in": "requests==2.31.0" };

  const scripts: ScriptInput[] = [
    { scriptContent: "print(1)", language: "python3", remotePath: "a", rawWorkspaceDependencies: deps },
    { scriptContent: "print(2)", language: "python3", remotePath: "b", rawWorkspaceDependencies: deps },
    { scriptContent: "print(1)", language: "python3", remotePath: "c", rawWorkspaceDependencies: deps },
    { scriptContent: "print(2)", language: "python3", remotePath: "d", rawWorkspaceDependencies: deps },
    { scriptContent: "print(3)", language: "python3", remotePath: "e", rawWorkspaceDependencies: deps },
  ];

  for (const s of scripts) await fetchScriptLockOld(s, remoteFn);
  assertEquals(callCount(), 5);
});

Deno.test("new logic: 5 scripts same deps → 1 remote call (all share cache)", async () => {
  const { remoteFn, callCount } = makeRemoteFn();
  const cache = new Map<string, string>();
  const deps = { "dependencies/requirements.in": "requests==2.31.0" };

  const scripts: ScriptInput[] = [
    { scriptContent: "print(1)", language: "python3", remotePath: "a", rawWorkspaceDependencies: deps },
    { scriptContent: "print(2)", language: "python3", remotePath: "b", rawWorkspaceDependencies: deps },
    { scriptContent: "print(1)", language: "python3", remotePath: "c", rawWorkspaceDependencies: deps },
    { scriptContent: "print(2)", language: "python3", remotePath: "d", rawWorkspaceDependencies: deps },
    { scriptContent: "print(3)", language: "python3", remotePath: "e", rawWorkspaceDependencies: deps },
  ];

  const results: string[] = [];
  for (const s of scripts) results.push(await fetchScriptLockNew(s, remoteFn, cache));
  assertEquals(callCount(), 1);
  // All scripts get the same cached lock
  for (let i = 1; i < results.length; i++) {
    assertEquals(results[0], results[i]);
  }
});

// -- Many scripts, different deps -------------------------------------------

Deno.test("new logic: 4 scripts with 2 dep groups → 2 remote calls", async () => {
  const { remoteFn, callCount } = makeRemoteFn();
  const cache = new Map<string, string>();
  const depsA = { "dependencies/requirements.in": "requests==2.31.0" };
  const depsB = { "dependencies/requirements.in": "flask==3.0.0" };

  const scripts: ScriptInput[] = [
    { scriptContent: "print(1)", language: "python3", remotePath: "a", rawWorkspaceDependencies: depsA },
    { scriptContent: "print(2)", language: "python3", remotePath: "b", rawWorkspaceDependencies: depsB },
    { scriptContent: "print(3)", language: "python3", remotePath: "c", rawWorkspaceDependencies: depsA },
    { scriptContent: "print(4)", language: "python3", remotePath: "d", rawWorkspaceDependencies: depsB },
  ];

  const results: string[] = [];
  for (const s of scripts) results.push(await fetchScriptLockNew(s, remoteFn, cache));
  assertEquals(callCount(), 2);
  assertEquals(results[0], results[2]); // same depsA
  assertEquals(results[1], results[3]); // same depsB
  assertNotEquals(results[0], results[1]); // depsA ≠ depsB
});

// -- Scripts with no workspace deps (empty) ---------------------------------

Deno.test("new logic: identical scripts with empty deps → 2 remote calls (no caching without raw deps)", async () => {
  const { remoteFn, callCount } = makeRemoteFn();
  const cache = new Map<string, string>();

  const scripts: ScriptInput[] = [
    { scriptContent: "print(1)", language: "python3", remotePath: "a", rawWorkspaceDependencies: {} },
    { scriptContent: "print(1)", language: "python3", remotePath: "b", rawWorkspaceDependencies: {} },
  ];

  for (const s of scripts) await fetchScriptLockNew(s, remoteFn, cache);
  assertEquals(callCount(), 2);
  assertEquals(cache.size, 0);
});

// -- Cache returns correct lock value, not stale data -----------------------

Deno.test("new logic: cached value matches original remote response", async () => {
  const cache = new Map<string, string>();
  const deps = { "dependencies/requirements.in": "requests==2.31.0" };

  let callIdx = 0;
  const remoteFn = async (_input: ScriptInput) => {
    callIdx++;
    return "resolved-lock-content-abc123";
  };

  const r1 = await fetchScriptLockNew(
    { scriptContent: "print(1)", language: "python3", remotePath: "a", rawWorkspaceDependencies: deps },
    remoteFn, cache,
  );
  const r2 = await fetchScriptLockNew(
    { scriptContent: "print(2)", language: "python3", remotePath: "b", rawWorkspaceDependencies: deps },
    remoteFn, cache,
  );

  assertEquals(callIdx, 1);
  assertEquals(r1, "resolved-lock-content-abc123");
  assertEquals(r2, "resolved-lock-content-abc123");
});
