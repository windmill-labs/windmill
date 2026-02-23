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

import { expect, test } from "bun:test";
import { Buffer } from "node:buffer";

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
  return Buffer.from(await crypto.subtle.digest("SHA-256", buf)).toString("hex");
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

test("python: manual requirements with external refs + inline deps", () => {
  const code = `# requirements:  default,   base
#requests==2.31.0
#pandas>=1.5.0

def main():
    pass`;
  const r = extractWorkspaceDepsAnnotation(code, "python3")!;
  expect(r.mode).toEqual("manual");
  expect(r.external).toEqual(["default", "base"]);
  expect(r.inline).toEqual("requests==2.31.0\npandas>=1.5.0");
});

test("python: extra_requirements mode", () => {
  const code = `# extra_requirements: utils
#numpy>=1.24.0

def main():
    pass`;
  const r = extractWorkspaceDepsAnnotation(code, "python3")!;
  expect(r.mode).toEqual("extra");
  expect(r.external).toEqual(["utils"]);
  expect(r.inline).toEqual("numpy>=1.24.0");
});

test("python: empty requirements (opt-out)", () => {
  const code = `# requirements:
def main():
    pass`;
  const r = extractWorkspaceDepsAnnotation(code, "python3")!;
  expect(r.mode).toEqual("manual");
  expect(r.external).toEqual([]);
  expect(r.inline).toEqual(null);
});

test("python: no annotation → null", () => {
  const code = `def main():
    print("hello")`;
  expect(extractWorkspaceDepsAnnotation(code, "python3")).toEqual(null);
});

test("bun: package_json annotation with inline", () => {
  const code = `// package_json: utils, base
//{
//  "dependencies": {
//    "axios": "^1.6.0"
//  }
//}

export function main() {}`;
  const r = extractWorkspaceDepsAnnotation(code, "bun")!;
  expect(r.mode).toEqual("manual");
  expect(r.external).toEqual(["utils", "base"]);
  expect(r.inline).toEqual(`{
  "dependencies": {
    "axios": "^1.6.0"
  }
}`);
});

test("go: go_mod annotation", () => {
  const code = `// go_mod:   base,
//github.com/gin-gonic/gin v1.9.1

package main
func main() {}`;
  const r = extractWorkspaceDepsAnnotation(code, "go")!;
  expect(r.mode).toEqual("manual");
  expect(r.external).toEqual(["base"]);
  expect(r.inline).toEqual("github.com/gin-gonic/gin v1.9.1");
});

test("unsupported language → null", () => {
  expect(extractWorkspaceDepsAnnotation("print(1)", "deno")).toEqual(null);
  expect(extractWorkspaceDepsAnnotation("print(1)", "bash")).toEqual(null);
});

// =============================================================================
// Part 2 — Cache key computation
// =============================================================================

test("same annotation + language + deps → same key", async () => {
  const code = `# requirements: default
#requests==2.31.0
print("hello")`;
  const deps = { "dependencies/requirements.in": "requests==2.31.0" };
  const a = await computeLockCacheKey(code, "python3", deps);
  const b = await computeLockCacheKey(code, "python3", deps);
  expect(a).toEqual(b);
});

test("different code, same annotation → same key", async () => {
  const codeA = `# requirements: default
#requests==2.31.0
print("hello")`;
  const codeB = `# requirements: default
#requests==2.31.0
print("world")`;
  const deps = { "dependencies/requirements.in": "requests==2.31.0" };
  expect(await computeLockCacheKey(codeA, "python3", deps)).toEqual(await computeLockCacheKey(codeB, "python3", deps));
});

test("different annotation inline → different key", async () => {
  const codeA = `# requirements: default
#requests==2.31.0
print("hello")`;
  const codeB = `# requirements: default
#flask==3.0.0
print("hello")`;
  const deps = { "dependencies/requirements.in": "requests==2.31.0" };
  expect(await computeLockCacheKey(codeA, "python3", deps)).not.toEqual(await computeLockCacheKey(codeB, "python3", deps));
});

test("different annotation external refs → different key", async () => {
  const codeA = `# requirements: default
print("hello")`;
  const codeB = `# requirements: base
print("hello")`;
  const deps = { "dependencies/requirements.in": "requests==2.31.0" };
  expect(await computeLockCacheKey(codeA, "python3", deps)).not.toEqual(await computeLockCacheKey(codeB, "python3", deps));
});

test("manual vs extra mode → different key", async () => {
  const codeA = `# requirements: default
print("hello")`;
  const codeB = `# extra_requirements: default
print("hello")`;
  const deps = { "dependencies/requirements.in": "requests==2.31.0" };
  expect(await computeLockCacheKey(codeA, "python3", deps)).not.toEqual(await computeLockCacheKey(codeB, "python3", deps));
});

test("no annotation, same code → same key", async () => {
  const deps = { "dependencies/requirements.in": "requests==2.31.0" };
  expect(await computeLockCacheKey("print('a')", "python3", deps)).toEqual(await computeLockCacheKey("print('b')", "python3", deps));
});

test("different deps → different key", async () => {
  const code = `# requirements: default
print("hello")`;
  expect(await computeLockCacheKey(code, "python3", { d: "a" })).not.toEqual(await computeLockCacheKey(code, "python3", { d: "b" }));
});

test("different language → different key", async () => {
  const deps = { d: "v" };
  expect(await computeLockCacheKey("x", "bun", deps)).not.toEqual(await computeLockCacheKey("x", "python3", deps));
});

test("dep key order does not matter", async () => {
  const code = "print('hello')";
  expect(await computeLockCacheKey(code, "python3", { a: "1", b: "2" })).toEqual(await computeLockCacheKey(code, "python3", { b: "2", a: "1" }));
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

test("old logic: two scripts same annotation → 2 remote calls", async () => {
  const { remoteFn, callCount } = makeRemoteFn();
  const deps = { "dependencies/requirements.in": "requests==2.31.0" };

  const scripts: ScriptInput[] = [
    { scriptContent: "# requirements: default\nprint(1)", language: "python3", remotePath: "a", rawWorkspaceDependencies: deps },
    { scriptContent: "# requirements: default\nprint(2)", language: "python3", remotePath: "b", rawWorkspaceDependencies: deps },
  ];

  for (const s of scripts) await fetchScriptLockOld(s, remoteFn);
  expect(callCount()).toEqual(2);
});

test("new logic: two scripts same annotation → 1 remote call (cache shared)", async () => {
  const { remoteFn, callCount } = makeRemoteFn();
  const cache = new Map<string, string>();
  const deps = { "dependencies/requirements.in": "requests==2.31.0" };

  const scripts: ScriptInput[] = [
    { scriptContent: "# requirements: default\nprint(1)", language: "python3", remotePath: "a", rawWorkspaceDependencies: deps },
    { scriptContent: "# requirements: default\nprint(2)", language: "python3", remotePath: "b", rawWorkspaceDependencies: deps },
  ];

  const results: string[] = [];
  for (const s of scripts) results.push(await fetchScriptLockNew(s, remoteFn, cache));
  expect(callCount()).toEqual(1);
  expect(results[0]).toEqual(results[1]);
});

// -- Two scripts, different annotations + same deps -------------------------

test("new logic: different annotations same deps → 2 remote calls", async () => {
  const { remoteFn, callCount } = makeRemoteFn();
  const cache = new Map<string, string>();
  const deps = { "dependencies/requirements.in": "requests==2.31.0" };

  const scripts: ScriptInput[] = [
    { scriptContent: "# requirements: default\nprint(1)", language: "python3", remotePath: "a", rawWorkspaceDependencies: deps },
    { scriptContent: "# requirements: base\nprint(2)", language: "python3", remotePath: "b", rawWorkspaceDependencies: deps },
  ];

  const results: string[] = [];
  for (const s of scripts) results.push(await fetchScriptLockNew(s, remoteFn, cache));
  expect(callCount()).toEqual(2);
  expect(results[0]).not.toEqual(results[1]);
});

// -- Two scripts, same annotation + different deps --------------------------

test("new logic: same annotation different deps → 2 remote calls", async () => {
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
  expect(callCount()).toEqual(2);
  expect(results[0]).not.toEqual(results[1]);
});

// -- Many scripts, same annotation + deps -----------------------------------

test("old logic: 5 scripts same annotation+deps → 5 remote calls", async () => {
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
  expect(callCount()).toEqual(5);
});

test("new logic: 5 scripts same annotation+deps → 1 remote call", async () => {
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
  expect(callCount()).toEqual(1);
  for (let i = 1; i < results.length; i++) {
    expect(results[0]).toEqual(results[i]);
  }
});

// -- Many scripts, 2 annotation groups + same deps -------------------------

test("new logic: 4 scripts with 2 annotation groups → 2 remote calls", async () => {
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
  expect(callCount()).toEqual(2);
  expect(results[0]).toEqual(results[2]); // same annotation "default"
  expect(results[1]).toEqual(results[3]); // same annotation "base"
  expect(results[0]).not.toEqual(results[1]);
});

// -- Scripts with no workspace deps (empty) ---------------------------------

test("new logic: empty deps → no caching", async () => {
  const { remoteFn, callCount } = makeRemoteFn();
  const cache = new Map<string, string>();

  const scripts: ScriptInput[] = [
    { scriptContent: "print(1)", language: "python3", remotePath: "a", rawWorkspaceDependencies: {} },
    { scriptContent: "print(1)", language: "python3", remotePath: "b", rawWorkspaceDependencies: {} },
  ];

  for (const s of scripts) await fetchScriptLockNew(s, remoteFn, cache);
  expect(callCount()).toEqual(2);
  expect(cache.size).toEqual(0);
});

// -- No annotation scripts with raw deps → share cache ---------------------

test("new logic: no annotation + same deps → 1 remote call", async () => {
  const { remoteFn, callCount } = makeRemoteFn();
  const cache = new Map<string, string>();
  const deps = { "dependencies/requirements.in": "requests==2.31.0" };

  const scripts: ScriptInput[] = [
    { scriptContent: "print(1)", language: "python3", remotePath: "a", rawWorkspaceDependencies: deps },
    { scriptContent: "print(2)", language: "python3", remotePath: "b", rawWorkspaceDependencies: deps },
  ];

  const results: string[] = [];
  for (const s of scripts) results.push(await fetchScriptLockNew(s, remoteFn, cache));
  expect(callCount()).toEqual(1);
  expect(results[0]).toEqual(results[1]);
});

// -- Mix of annotated and non-annotated scripts -----------------------------

test("new logic: mix of annotated and non-annotated → separate cache groups", async () => {
  const { remoteFn, callCount } = makeRemoteFn();
  const cache = new Map<string, string>();
  const deps = { "dependencies/requirements.in": "requests==2.31.0" };

  const scripts: ScriptInput[] = [
    { scriptContent: "# requirements: default\nprint(1)", language: "python3", remotePath: "a", rawWorkspaceDependencies: deps },
    { scriptContent: "print(2)", language: "python3", remotePath: "b", rawWorkspaceDependencies: deps },
    { scriptContent: "# requirements: default\nprint(3)", language: "python3", remotePath: "c", rawWorkspaceDependencies: deps },
    { scriptContent: "print(4)", language: "python3", remotePath: "d", rawWorkspaceDependencies: deps },
  ];

  const results: string[] = [];
  for (const s of scripts) results.push(await fetchScriptLockNew(s, remoteFn, cache));
  expect(callCount()).toEqual(2); // one for annotated group, one for no-annotation group
  expect(results[0]).toEqual(results[2]); // both annotated "default"
  expect(results[1]).toEqual(results[3]); // both no annotation
  expect(results[0]).not.toEqual(results[1]); // annotated ≠ non-annotated
});

// -- Cache returns correct lock value ---------------------------------------

test("new logic: cached value matches original remote response", async () => {
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

  expect(callIdx).toEqual(1);
  expect(r1).toEqual("resolved-lock-content-abc123");
  expect(r2).toEqual("resolved-lock-content-abc123");
});
