/**
 * Unit tests for failure_module and preprocessor_module inline script
 * extraction (pull) and replacement (push).
 *
 * These tests verify that rawscript content in failure_module and
 * preprocessor_module is correctly extracted to !inline references
 * and resolved back, matching the existing behavior for regular modules.
 */

import { expect, test, describe } from "bun:test";
import { extractInlineScripts, extractCurrentMapping } from "../windmill-utils-internal/src/inline-scripts/extractor.ts";
import { replaceInlineScripts } from "../windmill-utils-internal/src/inline-scripts/replacer.ts";
import { newPathAssigner } from "../windmill-utils-internal/src/path-utils/path-assigner.ts";
import type { FlowModule } from "../windmill-utils-internal/src/gen/types.gen.ts";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeRawscriptModule(
  id: string,
  content: string,
  language: "bun" | "python3" | "deno" = "bun",
  lock?: string,
): FlowModule {
  return {
    id,
    value: {
      type: "rawscript" as const,
      content,
      language,
      lock: lock,
      input_transforms: {},
    },
  };
}

const noopLogger = {
  info: () => {},
  error: () => {},
};

// ---------------------------------------------------------------------------
// extractInlineScripts — PULL direction
// ---------------------------------------------------------------------------

describe("extractInlineScripts for failure_module / preprocessor_module", () => {
  test("extracts rawscript from failure_module wrapped in array", () => {
    const failureModule = makeRawscriptModule(
      "failure",
      'export function main() { throw new Error("handler"); }',
      "bun",
    );

    const scripts = extractInlineScripts([failureModule], {}, "/", "bun");

    expect(scripts.length).toBeGreaterThanOrEqual(1);
    const script = scripts.find((s) => !s.is_lock);
    expect(script).toBeDefined();
    expect(script!.content).toBe(
      'export function main() { throw new Error("handler"); }',
    );
    // The module content should have been replaced with an !inline reference
    expect(failureModule.value.content).toStartWith("!inline ");
  });

  test("extracts rawscript from preprocessor_module wrapped in array", () => {
    const preprocessorModule = makeRawscriptModule(
      "preprocessor",
      "export function main() { return {}; }",
      "python3",
    );

    const scripts = extractInlineScripts(
      [preprocessorModule],
      {},
      "/",
      "bun",
    );

    expect(scripts.length).toBeGreaterThanOrEqual(1);
    const script = scripts.find((s) => !s.is_lock);
    expect(script).toBeDefined();
    expect(script!.content).toBe("export function main() { return {}; }");
    expect(script!.language).toBe("python3");
    expect(preprocessorModule.value.content).toStartWith("!inline ");
  });

  test("extracts lock alongside content", () => {
    const mod = makeRawscriptModule(
      "failure",
      "console.log('hi')",
      "bun",
      "some-lock-content",
    );

    const scripts = extractInlineScripts([mod], {}, "/", "bun");

    const contentScript = scripts.find((s) => !s.is_lock);
    const lockScript = scripts.find((s) => s.is_lock);
    expect(contentScript).toBeDefined();
    expect(lockScript).toBeDefined();
    expect(lockScript!.content).toBe("some-lock-content");
    expect((mod.value as any).lock).toStartWith("!inline ");
  });

  test("shared pathAssigner prevents collisions when summaries match", () => {
    // If a regular module and failure_module share the same summary,
    // a shared PathAssigner deduplicates via its internal counter.
    const regular = makeRawscriptModule("a", "code_a", "bun");
    regular.summary = "my step";
    const failure = makeRawscriptModule("failure", "code_failure", "bun");
    failure.summary = "my step"; // same summary — would collide without shared assigner

    const assigner = newPathAssigner("bun");
    const scripts1 = extractInlineScripts([regular], {}, "/", "bun", assigner);
    const scripts2 = extractInlineScripts([failure], {}, "/", "bun", assigner);

    const allPaths = [...scripts1, ...scripts2]
      .filter((s) => !s.is_lock)
      .map((s) => s.path);

    // All paths should be unique despite identical summaries
    expect(allPaths.length).toBe(2);
    expect(new Set(allPaths).size).toBe(2);
  });

  test("without shared pathAssigner, identical summaries produce duplicate paths", () => {
    // Demonstrates the problem that sharing a PathAssigner solves.
    const regular = makeRawscriptModule("a", "code_a", "bun");
    regular.summary = "my step";
    const failure = makeRawscriptModule("failure", "code_failure", "bun");
    failure.summary = "my step";

    // Separate assigners — each starts with a fresh counter
    const scripts1 = extractInlineScripts([regular], {}, "/", "bun");
    const scripts2 = extractInlineScripts([failure], {}, "/", "bun");

    const allPaths = [...scripts1, ...scripts2]
      .filter((s) => !s.is_lock)
      .map((s) => s.path);

    // Without a shared assigner, the paths collide
    expect(allPaths.length).toBe(2);
    expect(new Set(allPaths).size).toBe(1); // both got the same path
  });

  test("skips non-rawscript failure_module (identity type)", () => {
    const identityModule: FlowModule = {
      id: "failure",
      value: { type: "identity" as any },
    };
    const scripts = extractInlineScripts([identityModule], {}, "/", "bun");
    expect(scripts).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// replaceInlineScripts — PUSH direction
// ---------------------------------------------------------------------------

describe("replaceInlineScripts for failure_module / preprocessor_module", () => {
  test("resolves !inline reference back to file content", async () => {
    const failureModule = makeRawscriptModule(
      "failure",
      "!inline failure.inline_script.ts",
      "bun",
    );

    const files: Record<string, string> = {
      "failure.inline_script.ts": 'export function main() { return "error handled"; }',
    };

    await replaceInlineScripts(
      [failureModule],
      async (path) => {
        if (!(path in files)) throw new Error(`File not found: ${path}`);
        return files[path];
      },
      noopLogger,
      "/tmp/test/",
      "/",
    );

    expect(failureModule.value.content).toBe(
      'export function main() { return "error handled"; }',
    );
  });

  test("resolves !inline reference for preprocessor_module", async () => {
    const preprocessorModule = makeRawscriptModule(
      "preprocessor",
      "!inline preprocessor.inline_script.py",
      "python3",
    );

    const files: Record<string, string> = {
      "preprocessor.inline_script.py": "def main(): return {}",
    };

    await replaceInlineScripts(
      [preprocessorModule],
      async (path) => {
        if (!(path in files)) throw new Error(`File not found: ${path}`);
        return files[path];
      },
      noopLogger,
      "/tmp/test/",
      "/",
    );

    expect(preprocessorModule.value.content).toBe("def main(): return {}");
  });

  test("resolves !inline lock reference", async () => {
    const mod = makeRawscriptModule(
      "failure",
      "!inline failure.inline_script.ts",
      "bun",
      "!inline failure.inline_script.lock",
    );

    const files: Record<string, string> = {
      "failure.inline_script.ts": "code here",
      "failure.inline_script.lock": "lock-data-here",
    };

    await replaceInlineScripts(
      [mod],
      async (path) => {
        if (!(path in files)) throw new Error(`File not found: ${path}`);
        return files[path];
      },
      noopLogger,
      "/tmp/test/",
      "/",
    );

    expect(mod.value.content).toBe("code here");
    expect((mod.value as any).lock).toBe("lock-data-here");
  });

  test("leaves non-inline content untouched", async () => {
    const mod = makeRawscriptModule(
      "failure",
      "export function main() { return 1; }",
      "bun",
    );

    await replaceInlineScripts(
      [mod],
      async () => {
        throw new Error("should not be called");
      },
      noopLogger,
      "/tmp/test/",
      "/",
    );

    expect(mod.value.content).toBe("export function main() { return 1; }");
  });
});

// ---------------------------------------------------------------------------
// Round-trip: extract then replace
// ---------------------------------------------------------------------------

describe("round-trip extract → replace for failure_module / preprocessor_module", () => {
  test("failure_module content survives extract + replace", async () => {
    const originalContent = 'export function main(error: any) {\n  console.error(error);\n  return { handled: true };\n}';
    const failureModule = makeRawscriptModule(
      "failure",
      originalContent,
      "bun",
    );

    // PULL: extract inline scripts (mutates module in place)
    const extracted = extractInlineScripts([failureModule], {}, "/", "bun");
    expect(failureModule.value.content).toStartWith("!inline ");

    // Build a virtual filesystem from extracted scripts
    const files: Record<string, string> = {};
    for (const s of extracted) {
      files[s.path] = s.content;
    }

    // PUSH: replace inline references back
    await replaceInlineScripts(
      [failureModule],
      async (path) => {
        if (!(path in files)) throw new Error(`File not found: ${path}`);
        return files[path];
      },
      noopLogger,
      "/tmp/test/",
      "/",
    );

    expect(failureModule.value.content).toBe(originalContent);
  });

  test("preprocessor_module content survives extract + replace", async () => {
    const originalContent = "def main():\n    return {\"preprocessed\": True}";
    const preprocessorModule = makeRawscriptModule(
      "preprocessor",
      originalContent,
      "python3",
    );

    const extracted = extractInlineScripts(
      [preprocessorModule],
      {},
      "/",
      "bun",
    );
    expect(preprocessorModule.value.content).toStartWith("!inline ");

    const files: Record<string, string> = {};
    for (const s of extracted) {
      files[s.path] = s.content;
    }

    await replaceInlineScripts(
      [preprocessorModule],
      async (path) => {
        if (!(path in files)) throw new Error(`File not found: ${path}`);
        return files[path];
      },
      noopLogger,
      "/tmp/test/",
      "/",
    );

    expect(preprocessorModule.value.content).toBe(originalContent);
  });

  test("failure_module with lock survives extract + replace", async () => {
    const originalContent = "export function main() { return 42; }";
    const originalLock = "package-lock-contents-here";
    const mod = makeRawscriptModule(
      "failure",
      originalContent,
      "bun",
      originalLock,
    );

    const extracted = extractInlineScripts([mod], {}, "/", "bun");

    const files: Record<string, string> = {};
    for (const s of extracted) {
      files[s.path] = s.content;
    }

    await replaceInlineScripts(
      [mod],
      async (path) => {
        if (!(path in files)) throw new Error(`File not found: ${path}`);
        return files[path];
      },
      noopLogger,
      "/tmp/test/",
      "/",
    );

    expect(mod.value.content).toBe(originalContent);
    expect((mod.value as any).lock).toBe(originalLock);
  });

  test("full flow with modules + failure_module + preprocessor_module round-trips", async () => {
    const regularContent = "export function main() { return 'step1'; }";
    const failureContent = "export function main(e: any) { return e; }";
    const preprocessorContent = "def main():\n    pass";

    const modules = [makeRawscriptModule("a", regularContent, "bun")];
    const failureModule = makeRawscriptModule("failure", failureContent, "bun");
    const preprocessorModule = makeRawscriptModule("preprocessor", preprocessorContent, "python3");

    // Extract all (mimicking sync.ts pull logic)
    const allExtracted = [
      ...extractInlineScripts(modules, {}, "/", "bun"),
      ...extractInlineScripts([failureModule], {}, "/", "bun"),
      ...extractInlineScripts([preprocessorModule], {}, "/", "bun"),
    ];

    // All modules should now have !inline references
    expect(modules[0].value.content).toStartWith("!inline ");
    expect(failureModule.value.content).toStartWith("!inline ");
    expect(preprocessorModule.value.content).toStartWith("!inline ");

    // All paths should be unique
    const paths = allExtracted.filter((s) => !s.is_lock).map((s) => s.path);
    expect(new Set(paths).size).toBe(paths.length);

    // Build filesystem
    const files: Record<string, string> = {};
    for (const s of allExtracted) {
      files[s.path] = s.content;
    }

    const fileReader = async (path: string) => {
      if (!(path in files)) throw new Error(`File not found: ${path}`);
      return files[path];
    };

    // Replace all (mimicking flow.ts push logic)
    await replaceInlineScripts(modules, fileReader, noopLogger, "/tmp/", "/");
    await replaceInlineScripts([failureModule], fileReader, noopLogger, "/tmp/", "/");
    await replaceInlineScripts([preprocessorModule], fileReader, noopLogger, "/tmp/", "/");

    expect(modules[0].value.content).toBe(regularContent);
    expect(failureModule.value.content).toBe(failureContent);
    expect(preprocessorModule.value.content).toBe(preprocessorContent);
  });
});

// ---------------------------------------------------------------------------
// extractCurrentMapping
// ---------------------------------------------------------------------------

describe("extractCurrentMapping for failure_module / preprocessor_module", () => {
  test("extracts mapping from failure_module via optional param", () => {
    const failureModule: FlowModule = makeRawscriptModule(
      "failure",
      "!inline failure.inline_script.ts",
      "bun",
    );

    const mapping = extractCurrentMapping(
      undefined,
      {},
      failureModule,
      undefined,
    );

    expect(mapping["failure"]).toBe("failure.inline_script.ts");
  });

  test("extracts mapping from preprocessor_module via optional param", () => {
    const preprocessorModule: FlowModule = makeRawscriptModule(
      "preprocessor",
      "!inline preprocessor.inline_script.py",
      "python3",
    );

    const mapping = extractCurrentMapping(
      undefined,
      {},
      undefined,
      preprocessorModule,
    );

    expect(mapping["preprocessor"]).toBe("preprocessor.inline_script.py");
  });

  test("extracts mapping from modules + failure + preprocessor combined", () => {
    const modules: FlowModule[] = [
      makeRawscriptModule("a", "!inline a.inline_script.ts", "bun"),
    ];
    const failureModule = makeRawscriptModule(
      "failure",
      "!inline failure.inline_script.ts",
      "bun",
    );
    const preprocessorModule = makeRawscriptModule(
      "preprocessor",
      "!inline preprocessor.inline_script.py",
      "python3",
    );

    const mapping = extractCurrentMapping(
      modules,
      {},
      failureModule,
      preprocessorModule,
    );

    expect(mapping["a"]).toBe("a.inline_script.ts");
    expect(mapping["failure"]).toBe("failure.inline_script.ts");
    expect(mapping["preprocessor"]).toBe("preprocessor.inline_script.py");
  });

  test("ignores non-inline content in failure_module", () => {
    const failureModule = makeRawscriptModule(
      "failure",
      "export function main() {}",
      "bun",
    );

    const mapping = extractCurrentMapping(
      undefined,
      {},
      failureModule,
      undefined,
    );

    expect(mapping["failure"]).toBeUndefined();
  });
});

// ---------------------------------------------------------------------------
// extractInlineScripts with mapping — path preservation
// ---------------------------------------------------------------------------

describe("extractInlineScripts with mapping preserves file paths", () => {
  test("uses mapped path instead of assigner-generated path", () => {
    const mod = makeRawscriptModule("a", "console.log('hi')", "bun");
    mod.summary = "Get Users Data";

    const mapping = { a: "get_users.ts" };
    const scripts = extractInlineScripts([mod], mapping, "/", "bun");

    const contentScript = scripts.find((s) => !s.is_lock);
    expect(contentScript!.path).toBe("get_users.ts");
    // Module content should reference the mapped path
    expect(mod.value.content).toBe("!inline get_users.ts");
  });

  test("falls through to assigner when module ID not in mapping", () => {
    const mod = makeRawscriptModule("a", "console.log('hi')", "bun");
    mod.summary = "Get Users Data";

    const mapping = { other_id: "other.ts" };
    const scripts = extractInlineScripts([mod], mapping, "/", "bun");

    const contentScript = scripts.find((s) => !s.is_lock);
    // Should use assigner path based on summary, not mapped
    expect(contentScript!.path).toContain("get_users_data");
  });

  test("mapped modules and unmapped modules coexist", () => {
    const modA = makeRawscriptModule("a", "code_a", "bun");
    modA.summary = "Step A";
    const modB = makeRawscriptModule("b", "code_b", "bun");
    modB.summary = "Step B";

    const mapping = { a: "my_custom_name.ts" }; // only a is mapped
    const scripts = extractInlineScripts([modA, modB], mapping, "/", "bun");

    const paths = scripts.filter((s) => !s.is_lock).map((s) => s.path);
    expect(paths[0]).toBe("my_custom_name.ts");
    expect(paths[1]).toContain("step_b"); // assigner-generated from summary
  });

  test("lock path is derived from mapped content path", () => {
    const mod = makeRawscriptModule("a", "code", "bun", "lock-content");
    mod.summary = "Get Users Data";

    const mapping = { a: "get_users.ts" };
    const scripts = extractInlineScripts([mod], mapping, "/", "bun");

    const lockScript = scripts.find((s) => s.is_lock);
    expect(lockScript!.path).toBe("get_users.lock");
    expect((mod.value as any).lock).toBe("!inline get_users.lock");
  });

  test("lock path uses assigner basePath when no mapping", () => {
    const mod = makeRawscriptModule("a", "code", "bun", "lock-content");
    mod.summary = "Get Users Data";

    const scripts = extractInlineScripts([mod], {}, "/", "bun");

    const lockScript = scripts.find((s) => s.is_lock);
    expect(lockScript!.path).toContain("get_users_data");
    expect(lockScript!.path).toEndWith(".lock");
  });

  test("lock path handles dotted content paths correctly", () => {
    const mod = makeRawscriptModule("a", "code", "bun", "lock-content");

    const mapping = { a: "my.inline_script.ts" };
    const scripts = extractInlineScripts([mod], mapping, "/", "bun");

    const lockScript = scripts.find((s) => s.is_lock);
    expect(lockScript!.path).toBe("my.inline_script.lock");
  });
});
