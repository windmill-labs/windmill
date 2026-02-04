/**
 * Lock Cache Tests
 *
 * Tests the in-memory lock cache used when fetching lockfiles for scripts with
 * raw_workspace_dependencies.
 *
 * Part 1: Unit tests for annotation parsing (mirrors backend).
 * Part 2: Unit tests for cache key computation.
 * Part 3: Behavioral tests comparing old logic (no cache, always fetches)
 *         vs new logic (caches by key, skips duplicate fetches).
 */

import {
  assertEquals,
  assertNotEquals,
} from "https://deno.land/std@0.224.0/assert/mod.ts";
import { encodeHex } from "https://deno.land/std@0.224.0/encoding/hex.ts";

// ---------------------------------------------------------------------------
// Mirrors extractWorkspaceDepsAnnotation + computeLockCacheKey from
// src/utils/metadata.ts so we can test the algorithm without pulling in the
// full (unresolvable-in-tests) module graph.
// ---------------------------------------------------------------------------

type AnnotationMode = "manual" | "extra";

interface WorkspaceDepsAnnotation {
  mode: AnnotationMode;
  external: string[];
  inline: string | null;
}

const LANG_ANNOTATION_CONFIG: Record<
  string,
  { comment: string; keyword: string; validityRe?: RegExp } | undefined
> = {
  python3: { comment: "#", keyword: "requirements", validityRe: /^#\s?(\S+)\s*$/ },
  bun: { comment: "//", keyword: "package_json" },
  nativets: { comment: "//", keyword: "package_json" },
  go: { comment: "//", keyword: "go_mod" },
  php: { comment: "//", keyword: "composer_json" },
};

function extractWorkspaceDepsAnnotation(
  scriptContent: string,
  language: string,
): WorkspaceDepsAnnotation | null {
  const config = LANG_ANNOTATION_CONFIG[language];
  if (!config) return null;

  const { comment, keyword, validityRe } = config;
  const extraMarker = `extra_${keyword}:`;
  const manualMarker = `${keyword}:`;

  const lines = scriptContent.split("\n");

  let pos = -1;
  for (let i = 0; i < lines.length; i++) {
    const l = lines[i];
    if (l.startsWith(comment) && (l.includes(extraMarker) || l.includes(manualMarker))) {
      pos = i;
      break;
    }
  }
  if (pos === -1) return null;

  const annotationLine = lines[pos];
  const mode: AnnotationMode = annotationLine.includes(extraMarker) ? "extra" : "manual";

  const marker = mode === "extra" ? extraMarker : manualMarker;
  const unparsed = annotationLine.replaceAll(marker, "").replaceAll(comment, "");
  const external = unparsed
    .split(",")
    .map((s) => s.trim())
    .filter((s) => s.length > 0);

  const inlineParts: string[] = [];
  for (let i = pos + 1; i < lines.length; i++) {
    const l = lines[i];
    if (validityRe) {
      const match = validityRe.exec(l);
      if (match && match[1]) {
        inlineParts.push(match[1]);
      } else {
        break;
      }
    } else {
      if (!l.startsWith(comment)) {
        break;
      }
      inlineParts.push(l.substring(comment.length));
    }
  }

  const inlineStr = inlineParts.join("\n");
  const inline = inlineStr.trim().length > 0 ? inlineStr : null;

  return { mode, external, inline };
}

async function computeLockCacheKey(
  scriptContent: string,
  language: string,
  rawWorkspaceDependencies: Record<string, string>,
): Promise<string> {
  const annotation = extractWorkspaceDepsAnnotation(scriptContent, language);
  const annotationStr = annotation
    ? `${annotation.mode}|${annotation.external.join(",")}|${annotation.inline ?? ""}`
    : "none";
  const sortedDepsKeys = Object.keys(rawWorkspaceDependencies).sort();
  const depsStr = sortedDepsKeys
    .map((k) => `${k}=${rawWorkspaceDependencies[k]}`)
    .join(";");
  const content = `${language}|${annotationStr}|${depsStr}`;
  const buf = new TextEncoder().encode(content);
  return encodeHex(await crypto.subtle.digest("SHA-256", buf));
}

// ---------------------------------------------------------------------------
// Helpers that mirror the two fetch strategies (old / new).
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
        input.scriptContent,
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
// Part 1 — Annotation parsing
// =============================================================================

Deno.test("python: manual requirements with external refs + inline deps", () => {
  const code = `# requirements:  default,   base
#requests==2.31.0
#pandas>=1.5.0

def main():
    pass`;
  const r = extractWorkspaceDepsAnnotation(code, "python3")!;
  assertEquals(r.mode, "manual");
  assertEquals(r.external, ["default", "base"]);
  assertEquals(r.inline, "requests==2.31.0\npandas>=1.5.0");
});

Deno.test("python: extra_requirements mode", () => {
  const code = `# extra_requirements: utils
#numpy>=1.24.0

def main():
    pass`;
  const r = extractWorkspaceDepsAnnotation(code, "python3")!;
  assertEquals(r.mode, "extra");
  assertEquals(r.external, ["utils"]);
  assertEquals(r.inline, "numpy>=1.24.0");
});

Deno.test("python: empty requirements (opt-out)", () => {
  const code = `# requirements:
def main():
    pass`;
  const r = extractWorkspaceDepsAnnotation(code, "python3")!;
  assertEquals(r.mode, "manual");
  assertEquals(r.external, []);
  assertEquals(r.inline, null);
});

Deno.test("python: no annotation → null", () => {
  const code = `def main():
    print("hello")`;
  assertEquals(extractWorkspaceDepsAnnotation(code, "python3"), null);
});

Deno.test("bun: package_json annotation with inline", () => {
  const code = `// package_json: utils, base
//{
//  "dependencies": {
//    "axios": "^1.6.0"
//  }
//}

export function main() {}`;
  const r = extractWorkspaceDepsAnnotation(code, "bun")!;
  assertEquals(r.mode, "manual");
  assertEquals(r.external, ["utils", "base"]);
  assertEquals(r.inline, `{
  "dependencies": {
    "axios": "^1.6.0"
  }
}`);
});

Deno.test("go: go_mod annotation", () => {
  const code = `// go_mod:   base,
//github.com/gin-gonic/gin v1.9.1

package main
func main() {}`;
  const r = extractWorkspaceDepsAnnotation(code, "go")!;
  assertEquals(r.mode, "manual");
  assertEquals(r.external, ["base"]);
  assertEquals(r.inline, "github.com/gin-gonic/gin v1.9.1");
});

Deno.test("unsupported language → null", () => {
  assertEquals(extractWorkspaceDepsAnnotation("print(1)", "deno"), null);
  assertEquals(extractWorkspaceDepsAnnotation("print(1)", "bash"), null);
});

// =============================================================================
// Part 2 — Cache key computation
// =============================================================================

Deno.test("same annotation + language + deps → same key", async () => {
  const code = `# requirements: default
#requests==2.31.0
print("hello")`;
  const deps = { "dependencies/requirements.in": "requests==2.31.0" };
  const a = await computeLockCacheKey(code, "python3", deps);
  const b = await computeLockCacheKey(code, "python3", deps);
  assertEquals(a, b);
});

Deno.test("different code, same annotation → same key", async () => {
  const codeA = `# requirements: default
#requests==2.31.0
print("hello")`;
  const codeB = `# requirements: default
#requests==2.31.0
print("world")`;
  const deps = { "dependencies/requirements.in": "requests==2.31.0" };
  assertEquals(
    await computeLockCacheKey(codeA, "python3", deps),
    await computeLockCacheKey(codeB, "python3", deps),
  );
});

Deno.test("different annotation inline → different key", async () => {
  const codeA = `# requirements: default
#requests==2.31.0
print("hello")`;
  const codeB = `# requirements: default
#flask==3.0.0
print("hello")`;
  const deps = { "dependencies/requirements.in": "requests==2.31.0" };
  assertNotEquals(
    await computeLockCacheKey(codeA, "python3", deps),
    await computeLockCacheKey(codeB, "python3", deps),
  );
});

Deno.test("different annotation external refs → different key", async () => {
  const codeA = `# requirements: default
print("hello")`;
  const codeB = `# requirements: base
print("hello")`;
  const deps = { "dependencies/requirements.in": "requests==2.31.0" };
  assertNotEquals(
    await computeLockCacheKey(codeA, "python3", deps),
    await computeLockCacheKey(codeB, "python3", deps),
  );
});

Deno.test("manual vs extra mode → different key", async () => {
  const codeA = `# requirements: default
print("hello")`;
  const codeB = `# extra_requirements: default
print("hello")`;
  const deps = { "dependencies/requirements.in": "requests==2.31.0" };
  assertNotEquals(
    await computeLockCacheKey(codeA, "python3", deps),
    await computeLockCacheKey(codeB, "python3", deps),
  );
});

Deno.test("no annotation, same code → same key", async () => {
  const deps = { "dependencies/requirements.in": "requests==2.31.0" };
  assertEquals(
    await computeLockCacheKey("print('a')", "python3", deps),
    await computeLockCacheKey("print('b')", "python3", deps),
  );
});

Deno.test("different deps → different key", async () => {
  const code = `# requirements: default
print("hello")`;
  assertNotEquals(
    await computeLockCacheKey(code, "python3", { d: "a" }),
    await computeLockCacheKey(code, "python3", { d: "b" }),
  );
});

Deno.test("different language → different key", async () => {
  const deps = { d: "v" };
  assertNotEquals(
    await computeLockCacheKey("x", "bun", deps),
    await computeLockCacheKey("x", "python3", deps),
  );
});

Deno.test("dep key order does not matter", async () => {
  const code = "print('hello')";
  assertEquals(
    await computeLockCacheKey(code, "python3", { a: "1", b: "2" }),
    await computeLockCacheKey(code, "python3", { b: "2", a: "1" }),
  );
});

// =============================================================================
// Part 3 — Multi-script fetch behavior: old logic vs new logic
// =============================================================================

function makeRemoteFn(): {
  remoteFn: (input: ScriptInput) => Promise<string>;
  callCount: () => number;
} {
  const calls: ScriptInput[] = [];
  return {
    remoteFn: async (input: ScriptInput) => {
      calls.push(input);
      const depsStr = Object.entries(input.rawWorkspaceDependencies).sort().map(([k,v]) => `${k}=${v}`).join(",");
      return `lock for ${input.language}:${input.scriptContent}:${depsStr}`;
    },
    callCount: () => calls.length,
  };
}

// -- Two scripts, same annotation + language + deps -------------------------

Deno.test("old logic: two scripts same annotation → 2 remote calls", async () => {
  const { remoteFn, callCount } = makeRemoteFn();
  const deps = { "dependencies/requirements.in": "requests==2.31.0" };

  const scripts: ScriptInput[] = [
    { scriptContent: "# requirements: default\nprint(1)", language: "python3", remotePath: "a", rawWorkspaceDependencies: deps },
    { scriptContent: "# requirements: default\nprint(2)", language: "python3", remotePath: "b", rawWorkspaceDependencies: deps },
  ];

  for (const s of scripts) await fetchScriptLockOld(s, remoteFn);
  assertEquals(callCount(), 2);
});

Deno.test("new logic: two scripts same annotation → 1 remote call (cache shared)", async () => {
  const { remoteFn, callCount } = makeRemoteFn();
  const cache = new Map<string, string>();
  const deps = { "dependencies/requirements.in": "requests==2.31.0" };

  const scripts: ScriptInput[] = [
    { scriptContent: "# requirements: default\nprint(1)", language: "python3", remotePath: "a", rawWorkspaceDependencies: deps },
    { scriptContent: "# requirements: default\nprint(2)", language: "python3", remotePath: "b", rawWorkspaceDependencies: deps },
  ];

  const results: string[] = [];
  for (const s of scripts) results.push(await fetchScriptLockNew(s, remoteFn, cache));
  assertEquals(callCount(), 1);
  assertEquals(results[0], results[1]);
});

// -- Two scripts, different annotations + same deps -------------------------

Deno.test("new logic: different annotations same deps → 2 remote calls", async () => {
  const { remoteFn, callCount } = makeRemoteFn();
  const cache = new Map<string, string>();
  const deps = { "dependencies/requirements.in": "requests==2.31.0" };

  const scripts: ScriptInput[] = [
    { scriptContent: "# requirements: default\nprint(1)", language: "python3", remotePath: "a", rawWorkspaceDependencies: deps },
    { scriptContent: "# requirements: base\nprint(2)", language: "python3", remotePath: "b", rawWorkspaceDependencies: deps },
  ];

  const results: string[] = [];
  for (const s of scripts) results.push(await fetchScriptLockNew(s, remoteFn, cache));
  assertEquals(callCount(), 2);
  assertNotEquals(results[0], results[1]);
});

// -- Two scripts, same annotation + different deps --------------------------

Deno.test("new logic: same annotation different deps → 2 remote calls", async () => {
  const { remoteFn, callCount } = makeRemoteFn();
  const cache = new Map<string, string>();

  const scripts: ScriptInput[] = [
    { scriptContent: "# requirements: default\nprint(1)", language: "python3", remotePath: "a",
      rawWorkspaceDependencies: { "dependencies/requirements.in": "requests==2.31.0" } },
    { scriptContent: "# requirements: default\nprint(1)", language: "python3", remotePath: "b",
      rawWorkspaceDependencies: { "dependencies/requirements.in": "requests==2.32.0" } },
  ];

  const results: string[] = [];
  for (const s of scripts) results.push(await fetchScriptLockNew(s, remoteFn, cache));
  assertEquals(callCount(), 2);
  assertNotEquals(results[0], results[1]);
});

// -- Many scripts, same annotation + deps -----------------------------------

Deno.test("old logic: 5 scripts same annotation+deps → 5 remote calls", async () => {
  const { remoteFn, callCount } = makeRemoteFn();
  const deps = { "dependencies/requirements.in": "requests==2.31.0" };
  const ann = "# requirements: default\n";

  const scripts: ScriptInput[] = [
    { scriptContent: ann + "print(1)", language: "python3", remotePath: "a", rawWorkspaceDependencies: deps },
    { scriptContent: ann + "print(2)", language: "python3", remotePath: "b", rawWorkspaceDependencies: deps },
    { scriptContent: ann + "print(3)", language: "python3", remotePath: "c", rawWorkspaceDependencies: deps },
    { scriptContent: ann + "print(1)", language: "python3", remotePath: "d", rawWorkspaceDependencies: deps },
    { scriptContent: ann + "print(2)", language: "python3", remotePath: "e", rawWorkspaceDependencies: deps },
  ];

  for (const s of scripts) await fetchScriptLockOld(s, remoteFn);
  assertEquals(callCount(), 5);
});

Deno.test("new logic: 5 scripts same annotation+deps → 1 remote call", async () => {
  const { remoteFn, callCount } = makeRemoteFn();
  const cache = new Map<string, string>();
  const deps = { "dependencies/requirements.in": "requests==2.31.0" };
  const ann = "# requirements: default\n";

  const scripts: ScriptInput[] = [
    { scriptContent: ann + "print(1)", language: "python3", remotePath: "a", rawWorkspaceDependencies: deps },
    { scriptContent: ann + "print(2)", language: "python3", remotePath: "b", rawWorkspaceDependencies: deps },
    { scriptContent: ann + "print(3)", language: "python3", remotePath: "c", rawWorkspaceDependencies: deps },
    { scriptContent: ann + "print(1)", language: "python3", remotePath: "d", rawWorkspaceDependencies: deps },
    { scriptContent: ann + "print(2)", language: "python3", remotePath: "e", rawWorkspaceDependencies: deps },
  ];

  const results: string[] = [];
  for (const s of scripts) results.push(await fetchScriptLockNew(s, remoteFn, cache));
  assertEquals(callCount(), 1);
  for (let i = 1; i < results.length; i++) {
    assertEquals(results[0], results[i]);
  }
});

// -- Many scripts, 2 annotation groups + same deps -------------------------

Deno.test("new logic: 4 scripts with 2 annotation groups → 2 remote calls", async () => {
  const { remoteFn, callCount } = makeRemoteFn();
  const cache = new Map<string, string>();
  const deps = { "dependencies/requirements.in": "requests==2.31.0" };

  const scripts: ScriptInput[] = [
    { scriptContent: "# requirements: default\nprint(1)", language: "python3", remotePath: "a", rawWorkspaceDependencies: deps },
    { scriptContent: "# requirements: base\nprint(2)", language: "python3", remotePath: "b", rawWorkspaceDependencies: deps },
    { scriptContent: "# requirements: default\nprint(3)", language: "python3", remotePath: "c", rawWorkspaceDependencies: deps },
    { scriptContent: "# requirements: base\nprint(4)", language: "python3", remotePath: "d", rawWorkspaceDependencies: deps },
  ];

  const results: string[] = [];
  for (const s of scripts) results.push(await fetchScriptLockNew(s, remoteFn, cache));
  assertEquals(callCount(), 2);
  assertEquals(results[0], results[2]); // same annotation "default"
  assertEquals(results[1], results[3]); // same annotation "base"
  assertNotEquals(results[0], results[1]);
});

// -- Scripts with no workspace deps (empty) ---------------------------------

Deno.test("new logic: empty deps → no caching", async () => {
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

// -- No annotation scripts with raw deps → share cache ---------------------

Deno.test("new logic: no annotation + same deps → 1 remote call", async () => {
  const { remoteFn, callCount } = makeRemoteFn();
  const cache = new Map<string, string>();
  const deps = { "dependencies/requirements.in": "requests==2.31.0" };

  const scripts: ScriptInput[] = [
    { scriptContent: "print(1)", language: "python3", remotePath: "a", rawWorkspaceDependencies: deps },
    { scriptContent: "print(2)", language: "python3", remotePath: "b", rawWorkspaceDependencies: deps },
  ];

  const results: string[] = [];
  for (const s of scripts) results.push(await fetchScriptLockNew(s, remoteFn, cache));
  assertEquals(callCount(), 1);
  assertEquals(results[0], results[1]);
});

// -- Cache returns correct lock value ---------------------------------------

Deno.test("new logic: cached value matches original remote response", async () => {
  const cache = new Map<string, string>();
  const deps = { "dependencies/requirements.in": "requests==2.31.0" };

  let callIdx = 0;
  const remoteFn = async (_input: ScriptInput) => {
    callIdx++;
    return "resolved-lock-content-abc123";
  };

  const r1 = await fetchScriptLockNew(
    { scriptContent: "# requirements: default\nprint(1)", language: "python3", remotePath: "a", rawWorkspaceDependencies: deps },
    remoteFn, cache,
  );
  const r2 = await fetchScriptLockNew(
    { scriptContent: "# requirements: default\nprint(2)", language: "python3", remotePath: "b", rawWorkspaceDependencies: deps },
    remoteFn, cache,
  );

  assertEquals(callIdx, 1);
  assertEquals(r1, "resolved-lock-content-abc123");
  assertEquals(r2, "resolved-lock-content-abc123");
});
