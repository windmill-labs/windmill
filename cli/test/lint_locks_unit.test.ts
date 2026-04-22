import { expect, test, describe } from "bun:test";
import { mkdtemp, rm, mkdir, writeFile } from "node:fs/promises";
import os from "node:os";
import * as path from "node:path";
import { checkMissingLocks, runLint } from "../src/commands/lint/lint.ts";

async function withTempDir(
  fn: (tempDir: string) => Promise<void>,
): Promise<void> {
  const tempDir = await mkdtemp(path.join(os.tmpdir(), "wmill_lint_locks_"));
  const originalCwd = process.cwd();
  try {
    process.chdir(tempDir);
    await fn(tempDir);
  } finally {
    process.chdir(originalCwd);
    await rm(tempDir, { recursive: true });
  }
}

// Helper to create a script with metadata and optional lock
async function createScript(
  tempDir: string,
  scriptBase: string,
  ext: string,
  opts: { lock?: string; lockFileContent?: string } = {},
) {
  const dir = path.dirname(path.join(tempDir, scriptBase));
  await mkdir(dir, { recursive: true });

  // Script content file
  await writeFile(path.join(tempDir, scriptBase + ext), "# placeholder", "utf-8");

  // Metadata YAML
  const lockLine = opts.lock !== undefined ? `lock: "${opts.lock}"` : "lock: ''";
  await writeFile(
    path.join(tempDir, scriptBase + ".script.yaml"),
    `summary: test\n${lockLine}\nschema:\n  properties: {}\n`,
    "utf-8",
  );

  // Lock file (if inline reference)
  if (opts.lockFileContent !== undefined) {
    await writeFile(
      path.join(tempDir, scriptBase + ".script.lock"),
      opts.lockFileContent,
      "utf-8",
    );
  }
}

// --- checkMissingLocks unit tests ---

describe("checkMissingLocks", () => {
  test("reports missing lock for python script", async () => {
    await withTempDir(async (tempDir) => {
      await createScript(tempDir, "f/my_script", ".py", { lock: "" });

      const issues = await checkMissingLocks({} as any, tempDir);

      expect(issues.length).toBe(1);
      expect(issues[0].target).toBe("script");
      expect(issues[0].errors[0]).toContain("Missing lock");
      expect(issues[0].errors[0]).toContain("python3");
    });
  });

  test("no issues for python script with inline lock file", async () => {
    await withTempDir(async (tempDir) => {
      await createScript(tempDir, "f/my_script", ".py", {
        lock: "!inline f/my_script.script.lock",
        lockFileContent: "some-dep==1.0.0",
      });

      const issues = await checkMissingLocks({} as any, tempDir);

      expect(issues.length).toBe(0);
    });
  });

  test("reports missing lock when inline lock file is empty", async () => {
    await withTempDir(async (tempDir) => {
      await createScript(tempDir, "f/my_script", ".py", {
        lock: "!inline f/my_script.script.lock",
        lockFileContent: "",
      });

      const issues = await checkMissingLocks({} as any, tempDir);

      expect(issues.length).toBe(1);
      expect(issues[0].errors[0]).toContain("Missing lock");
    });
  });

  test("no issues for bash script without lock (lock not required)", async () => {
    await withTempDir(async (tempDir) => {
      await createScript(tempDir, "f/my_bash", ".sh", { lock: "" });

      const issues = await checkMissingLocks({} as any, tempDir);

      expect(issues.length).toBe(0);
    });
  });

  test("reports missing lock for bun script", async () => {
    await withTempDir(async (tempDir) => {
      await createScript(tempDir, "f/my_ts", ".ts", { lock: "" });

      const issues = await checkMissingLocks(
        { defaultTs: "bun" } as any,
        tempDir,
      );

      expect(issues.length).toBe(1);
      expect(issues[0].errors[0]).toContain("Missing lock");
      expect(issues[0].errors[0]).toContain("bun");
    });
  });

  test("reports missing lock for flow inline rawscript", async () => {
    await withTempDir(async (tempDir) => {
      await mkdir(`${tempDir}/f/my_flow.flow`, { recursive: true });
      await writeFile(
        `${tempDir}/f/my_flow.flow/flow.yaml`,
        `summary: test flow
value:
  modules:
    - id: step1
      value:
        type: rawscript
        language: python3
        content: "print('hello')"
`,
        "utf-8",
      );

      const issues = await checkMissingLocks({} as any, tempDir);

      expect(issues.length).toBe(1);
      expect(issues[0].target).toBe("flow_inline_script");
      expect(issues[0].errors[0]).toContain("step1");
      expect(issues[0].errors[0]).toContain("python3");
    });
  });

  test("no issues for flow inline rawscript with lock", async () => {
    await withTempDir(async (tempDir) => {
      await mkdir(`${tempDir}/f/my_flow.flow`, { recursive: true });
      await writeFile(
        `${tempDir}/f/my_flow.flow/flow.yaml`,
        `summary: test flow
value:
  modules:
    - id: step1
      value:
        type: rawscript
        language: python3
        content: "print('hello')"
        lock: "some-dep==1.0.0"
`,
        "utf-8",
      );

      const issues = await checkMissingLocks({} as any, tempDir);

      expect(issues.length).toBe(0);
    });
  });

  test("reports missing lock for nested flow modules (forloopflow)", async () => {
    await withTempDir(async (tempDir) => {
      await mkdir(`${tempDir}/f/my_flow.flow`, { recursive: true });
      await writeFile(
        `${tempDir}/f/my_flow.flow/flow.yaml`,
        `summary: test flow
value:
  modules:
    - id: loop1
      value:
        type: forloopflow
        modules:
          - id: inner_step
            value:
              type: rawscript
              language: python3
              content: "print('inner')"
`,
        "utf-8",
      );

      const issues = await checkMissingLocks({} as any, tempDir);

      expect(issues.length).toBe(1);
      expect(issues[0].errors[0]).toContain("inner_step");
    });
  });

  test("reports missing lock for app inline script", async () => {
    await withTempDir(async (tempDir) => {
      await mkdir(`${tempDir}/f/my_app.app`, { recursive: true });
      await writeFile(
        `${tempDir}/f/my_app.app/app.yaml`,
        `value:
  grid:
    - data:
        inlineScript:
          language: python3
          content: "x = 1"
`,
        "utf-8",
      );

      const issues = await checkMissingLocks({} as any, tempDir);

      expect(issues.length).toBe(1);
      expect(issues[0].target).toBe("app_inline_script");
      expect(issues[0].errors[0]).toContain("python3");
    });
  });

  test("no issues for app inline script with lock", async () => {
    await withTempDir(async (tempDir) => {
      await mkdir(`${tempDir}/f/my_app.app`, { recursive: true });
      await writeFile(
        `${tempDir}/f/my_app.app/app.yaml`,
        `value:
  grid:
    - data:
        inlineScript:
          language: python3
          content: "x = 1"
          lock: "some-dep==1.0.0"
`,
        "utf-8",
      );

      const issues = await checkMissingLocks({} as any, tempDir);

      expect(issues.length).toBe(0);
    });
  });

  test("no issues for flow with non-lock-requiring language (bash)", async () => {
    await withTempDir(async (tempDir) => {
      await mkdir(`${tempDir}/f/my_flow.flow`, { recursive: true });
      await writeFile(
        `${tempDir}/f/my_flow.flow/flow.yaml`,
        `summary: test flow
value:
  modules:
    - id: step1
      value:
        type: rawscript
        language: bash
        content: "echo hello"
`,
        "utf-8",
      );

      const issues = await checkMissingLocks({} as any, tempDir);

      expect(issues.length).toBe(0);
    });
  });

  test("skips raw app without backend folder", async () => {
    await withTempDir(async (tempDir) => {
      await mkdir(`${tempDir}/f/my_rawapp.raw_app`, { recursive: true });
      await writeFile(
        `${tempDir}/f/my_rawapp.raw_app/raw_app.yaml`,
        `summary: test raw app
`,
        "utf-8",
      );
      // No backend/ folder created

      const issues = await checkMissingLocks({} as any, tempDir);

      expect(issues.length).toBe(0);
    });
  });
});

// --- runLint --locks-required integration tests ---

describe("runLint with --locks-required", () => {
  test("reports lock issues when locksRequired is true", async () => {
    await withTempDir(async (tempDir) => {
      await createScript(tempDir, "f/my_script", ".py", { lock: "" });

      const report = await runLint({ locksRequired: true } as any, tempDir);

      expect(report.success).toBe(false);
      expect(report.exitCode).toBe(1);
      expect(report.issues.length).toBeGreaterThanOrEqual(1);
      expect(
        report.issues.some((i) => i.errors.some((e) => e.includes("Missing lock"))),
      ).toBe(true);
    });
  });

  test("does not check locks when locksRequired is false", async () => {
    await withTempDir(async (tempDir) => {
      await createScript(tempDir, "f/my_script", ".py", { lock: "" });

      const report = await runLint({} as any, tempDir);

      // Without locksRequired, no lock issues should appear
      expect(
        report.issues.some((i) => i.errors.some((e) => e.includes("Missing lock"))),
      ).toBe(false);
    });
  });

  test("passes when locksRequired is true and locks exist", async () => {
    await withTempDir(async (tempDir) => {
      await createScript(tempDir, "f/my_script", ".py", {
        lock: "!inline f/my_script.script.lock",
        lockFileContent: "some-dep==1.0.0",
      });

      const report = await runLint({ locksRequired: true } as any, tempDir);

      expect(report.success).toBe(true);
      expect(report.exitCode).toBe(0);
      expect(
        report.issues.some((i) => i.errors.some((e) => e.includes("Missing lock"))),
      ).toBe(false);
    });
  });
});
