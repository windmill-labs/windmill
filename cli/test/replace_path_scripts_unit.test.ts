/**
 * Unit tests for replacePathScriptsWithLocal.
 *
 * Tests that PathScript ("script" type) modules are correctly converted
 * to RawScript ("rawscript" type) using local file content during
 * flow preview / dev mode.
 */

import { expect, test, describe } from "bun:test";
import { replacePathScriptsWithLocal, type LocalScriptInfo } from "../windmill-utils-internal/src/inline-scripts/replacer.ts";
import type { FlowModule } from "../windmill-utils-internal/src/gen/types.gen.ts";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makePathScriptModule(
  id: string,
  path: string,
  inputTransforms: Record<string, any> = {},
  tagOverride?: string,
): FlowModule {
  return {
    id,
    value: {
      type: "script" as const,
      path,
      input_transforms: inputTransforms,
      tag_override: tagOverride,
    },
  };
}

function makeRawscriptModule(
  id: string,
  content: string,
  language: "bun" | "python3" | "deno" = "bun",
): FlowModule {
  return {
    id,
    value: {
      type: "rawscript" as const,
      content,
      language,
      input_transforms: {},
    },
  };
}

const noopLogger = {
  info: () => {},
  error: () => {},
};

// ---------------------------------------------------------------------------
// Basic conversion tests
// ---------------------------------------------------------------------------

describe("replacePathScriptsWithLocal", () => {
  test("converts PathScript to RawScript when local file exists", async () => {
    const module = makePathScriptModule("a", "f/scripts/my_script", {
      x: { type: "static", value: 42 },
    });

    const scriptReader = async (scriptPath: string): Promise<LocalScriptInfo | undefined> => {
      if (scriptPath === "f/scripts/my_script") {
        return {
          content: 'export function main() { return "hello"; }',
          language: "bun",
          lock: "some-lock",
        };
      }
      return undefined;
    };

    await replacePathScriptsWithLocal([module], scriptReader, noopLogger);

    expect(module.value.type).toBe("rawscript");
    expect((module.value as any).content).toBe('export function main() { return "hello"; }');
    expect((module.value as any).language).toBe("bun");
    expect((module.value as any).lock).toBe("some-lock");
    expect((module.value as any).path).toBe("f/scripts/my_script");
    expect((module.value as any).input_transforms).toEqual({ x: { type: "static", value: 42 } });
  });

  test("preserves tag_override as tag", async () => {
    const module = makePathScriptModule("a", "f/scripts/tagged", {}, "my-worker");

    const scriptReader = async (scriptPath: string): Promise<LocalScriptInfo | undefined> => {
      if (scriptPath === "f/scripts/tagged") {
        return { content: "code", language: "python3", tag: "script-worker" };
      }
      return undefined;
    };

    await replacePathScriptsWithLocal([module], scriptReader, noopLogger);

    expect(module.value.type).toBe("rawscript");
    expect((module.value as any).tag).toBe("my-worker");
  });

  test("uses local script tag when no tag_override is set", async () => {
    const module = makePathScriptModule("a", "f/scripts/tagged");

    const scriptReader = async (scriptPath: string): Promise<LocalScriptInfo | undefined> => {
      if (scriptPath === "f/scripts/tagged") {
        return { content: "code", language: "python3", tag: "script-worker" };
      }
      return undefined;
    };

    await replacePathScriptsWithLocal([module], scriptReader, noopLogger);

    expect(module.value.type).toBe("rawscript");
    expect((module.value as any).tag).toBe("script-worker");
  });

  test("leaves PathScript untouched when local file not found", async () => {
    const module = makePathScriptModule("a", "f/scripts/remote_only");

    const scriptReader = async (): Promise<LocalScriptInfo | undefined> => undefined;

    await replacePathScriptsWithLocal([module], scriptReader, noopLogger);

    expect(module.value.type).toBe("script");
    expect((module.value as any).path).toBe("f/scripts/remote_only");
  });

  test("does not affect rawscript modules", async () => {
    const module = makeRawscriptModule("a", "existing code", "bun");

    const scriptReader = async (): Promise<LocalScriptInfo | undefined> => {
      throw new Error("should not be called for rawscript");
    };

    await replacePathScriptsWithLocal([module], scriptReader, noopLogger);

    expect(module.value.type).toBe("rawscript");
    expect((module.value as any).content).toBe("existing code");
  });

  test("handles mixed PathScript and RawScript modules", async () => {
    const pathModule = makePathScriptModule("a", "f/scripts/local_script");
    const rawModule = makeRawscriptModule("b", "inline code", "bun");
    const remoteModule = makePathScriptModule("c", "f/scripts/remote_only");

    const scriptReader = async (scriptPath: string): Promise<LocalScriptInfo | undefined> => {
      if (scriptPath === "f/scripts/local_script") {
        return { content: "local code", language: "python3" };
      }
      return undefined;
    };

    await replacePathScriptsWithLocal(
      [pathModule, rawModule, remoteModule],
      scriptReader,
      noopLogger,
    );

    // PathScript with local file → converted
    expect(pathModule.value.type).toBe("rawscript");
    expect((pathModule.value as any).content).toBe("local code");

    // RawScript → untouched
    expect(rawModule.value.type).toBe("rawscript");
    expect((rawModule.value as any).content).toBe("inline code");

    // PathScript without local file → untouched
    expect(remoteModule.value.type).toBe("script");
    expect((remoteModule.value as any).path).toBe("f/scripts/remote_only");
  });

  test("handles module without lock", async () => {
    const module = makePathScriptModule("a", "f/scripts/no_lock");

    const scriptReader = async (scriptPath: string): Promise<LocalScriptInfo | undefined> => {
      if (scriptPath === "f/scripts/no_lock") {
        return { content: "code", language: "go" };
      }
      return undefined;
    };

    await replacePathScriptsWithLocal([module], scriptReader, noopLogger);

    expect(module.value.type).toBe("rawscript");
    expect((module.value as any).lock).toBeUndefined();
  });
});

// ---------------------------------------------------------------------------
// Nested structure tests
// ---------------------------------------------------------------------------

describe("replacePathScriptsWithLocal nested structures", () => {
  test("processes modules inside forloopflow", async () => {
    const innerModule = makePathScriptModule("inner", "f/scripts/loop_script");
    const loopModule: FlowModule = {
      id: "loop",
      value: {
        type: "forloopflow" as const,
        iterator: { type: "static" as const, value: "" },
        modules: [innerModule],
        skip_failures: false,
      },
    };

    const scriptReader = async (scriptPath: string): Promise<LocalScriptInfo | undefined> => {
      if (scriptPath === "f/scripts/loop_script") {
        return { content: "loop code", language: "bun" };
      }
      return undefined;
    };

    await replacePathScriptsWithLocal([loopModule], scriptReader, noopLogger);

    expect(innerModule.value.type).toBe("rawscript");
    expect((innerModule.value as any).content).toBe("loop code");
  });

  test("processes modules inside whileloopflow", async () => {
    const innerModule = makePathScriptModule("inner", "f/scripts/while_script");
    const whileModule: FlowModule = {
      id: "while",
      value: {
        type: "whileloopflow" as const,
        modules: [innerModule],
        skip_failures: false,
      },
    };

    const scriptReader = async (scriptPath: string): Promise<LocalScriptInfo | undefined> => {
      if (scriptPath === "f/scripts/while_script") {
        return { content: "while code", language: "bun" };
      }
      return undefined;
    };

    await replacePathScriptsWithLocal([whileModule], scriptReader, noopLogger);

    expect(innerModule.value.type).toBe("rawscript");
    expect((innerModule.value as any).content).toBe("while code");
  });

  test("processes modules inside branchall", async () => {
    const branch1Module = makePathScriptModule("b1", "f/scripts/branch1");
    const branch2Module = makePathScriptModule("b2", "f/scripts/branch2");
    const branchAllModule: FlowModule = {
      id: "branches",
      value: {
        type: "branchall" as const,
        branches: [
          { modules: [branch1Module], skip_failure: false },
          { modules: [branch2Module], skip_failure: false },
        ],
      },
    };

    const scriptReader = async (scriptPath: string): Promise<LocalScriptInfo | undefined> => {
      if (scriptPath === "f/scripts/branch1") {
        return { content: "branch1 code", language: "bun" };
      }
      if (scriptPath === "f/scripts/branch2") {
        return { content: "branch2 code", language: "python3" };
      }
      return undefined;
    };

    await replacePathScriptsWithLocal([branchAllModule], scriptReader, noopLogger);

    expect(branch1Module.value.type).toBe("rawscript");
    expect((branch1Module.value as any).content).toBe("branch1 code");
    expect(branch2Module.value.type).toBe("rawscript");
    expect((branch2Module.value as any).content).toBe("branch2 code");
    expect((branch2Module.value as any).language).toBe("python3");
  });

  test("processes modules inside branchone (branches + default)", async () => {
    const branchModule = makePathScriptModule("b1", "f/scripts/branch");
    const defaultModule = makePathScriptModule("d1", "f/scripts/default");
    const branchOneModule: FlowModule = {
      id: "branchone",
      value: {
        type: "branchone" as const,
        branches: [
          { modules: [branchModule], expr: "true" },
        ],
        default: [defaultModule],
      },
    };

    const scriptReader = async (scriptPath: string): Promise<LocalScriptInfo | undefined> => {
      if (scriptPath === "f/scripts/branch") {
        return { content: "branch code", language: "bun" };
      }
      if (scriptPath === "f/scripts/default") {
        return { content: "default code", language: "bash" };
      }
      return undefined;
    };

    await replacePathScriptsWithLocal([branchOneModule], scriptReader, noopLogger);

    expect(branchModule.value.type).toBe("rawscript");
    expect((branchModule.value as any).content).toBe("branch code");
    expect(defaultModule.value.type).toBe("rawscript");
    expect((defaultModule.value as any).content).toBe("default code");
    expect((defaultModule.value as any).language).toBe("bash");
  });

  test("handles empty modules array", async () => {
    const scriptReader = async (): Promise<LocalScriptInfo | undefined> => undefined;
    // Should not throw
    await replacePathScriptsWithLocal([], scriptReader, noopLogger);
  });

  test("handles module with undefined value gracefully", async () => {
    const module: FlowModule = { id: "x", value: undefined as any };
    const scriptReader = async (): Promise<LocalScriptInfo | undefined> => undefined;
    // Should not throw
    await replacePathScriptsWithLocal([module], scriptReader, noopLogger);
  });
});
