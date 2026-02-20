import {
  assert,
  assertEquals,
} from "https://deno.land/std@0.224.0/assert/mod.ts";
import {
  checkMissingLocks,
  runLint,
} from "../src/commands/lint/lint.ts";

async function withTempDir(
  fn: (tempDir: string) => Promise<void>,
): Promise<void> {
  const tempDir = await Deno.makeTempDir({ prefix: "wmill_locks_test_" });
  const originalCwd = Deno.cwd();
  try {
    Deno.chdir(tempDir);
    await fn(tempDir);
  } finally {
    Deno.chdir(originalCwd);
    await Deno.remove(tempDir, { recursive: true });
  }
}

// --- checkMissingLocks unit tests ---

Deno.test("locks-required: passes for python script with non-empty lock file", async () => {
  await withTempDir(async (tempDir) => {
    await Deno.mkdir(`${tempDir}/f/folder`, { recursive: true });

    await Deno.writeTextFile(
      `${tempDir}/f/folder/my_script.py`,
      `import pandas\ndef main(): pass`,
    );
    await Deno.writeTextFile(
      `${tempDir}/f/folder/my_script.script.yaml`,
      `summary: ""\ndescription: ""\nlock: "!inline f/folder/my_script.script.lock"\nkind: script\nschema:\n  $schema: "https://json-schema.org/draft/2020-12/schema"\n  type: object\n  properties: {}\n  required: []\n`,
    );
    await Deno.writeTextFile(
      `${tempDir}/f/folder/my_script.script.lock`,
      `pandas==2.0.0\nnumpy==1.24.0\n`,
    );

    const issues = await checkMissingLocks({} as any, tempDir);
    assertEquals(issues.length, 0);
  });
});

Deno.test("locks-required: fails for python script with empty lock file", async () => {
  await withTempDir(async (tempDir) => {
    await Deno.mkdir(`${tempDir}/f/folder`, { recursive: true });

    await Deno.writeTextFile(
      `${tempDir}/f/folder/my_script.py`,
      `def main(): pass`,
    );
    await Deno.writeTextFile(
      `${tempDir}/f/folder/my_script.script.yaml`,
      `summary: ""\ndescription: ""\nlock: "!inline f/folder/my_script.script.lock"\nkind: script\nschema:\n  $schema: "https://json-schema.org/draft/2020-12/schema"\n  type: object\n  properties: {}\n  required: []\n`,
    );
    await Deno.writeTextFile(
      `${tempDir}/f/folder/my_script.script.lock`,
      ``,
    );

    const issues = await checkMissingLocks({} as any, tempDir);
    assertEquals(issues.length, 1);
    assertEquals(issues[0].target, "script");
    assert(issues[0].errors[0].includes("Missing lock"));
    assert(issues[0].errors[0].includes("python3"));
  });
});

Deno.test("locks-required: fails for python script with missing lock file", async () => {
  await withTempDir(async (tempDir) => {
    await Deno.mkdir(`${tempDir}/f/folder`, { recursive: true });

    await Deno.writeTextFile(
      `${tempDir}/f/folder/my_script.py`,
      `def main(): pass`,
    );
    await Deno.writeTextFile(
      `${tempDir}/f/folder/my_script.script.yaml`,
      `summary: ""\ndescription: ""\nlock: "!inline f/folder/nonexistent.script.lock"\nkind: script\nschema:\n  $schema: "https://json-schema.org/draft/2020-12/schema"\n  type: object\n  properties: {}\n  required: []\n`,
    );

    const issues = await checkMissingLocks({} as any, tempDir);
    assertEquals(issues.length, 1);
    assertEquals(issues[0].target, "script");
  });
});

Deno.test("locks-required: fails for python script with lock field empty string", async () => {
  await withTempDir(async (tempDir) => {
    await Deno.mkdir(`${tempDir}/f/folder`, { recursive: true });

    await Deno.writeTextFile(
      `${tempDir}/f/folder/my_script.py`,
      `def main(): pass`,
    );
    await Deno.writeTextFile(
      `${tempDir}/f/folder/my_script.script.yaml`,
      `summary: ""\ndescription: ""\nlock: ""\nkind: script\nschema:\n  $schema: "https://json-schema.org/draft/2020-12/schema"\n  type: object\n  properties: {}\n  required: []\n`,
    );

    const issues = await checkMissingLocks({} as any, tempDir);
    assertEquals(issues.length, 1);
    assertEquals(issues[0].target, "script");
  });
});

Deno.test("locks-required: skips bash scripts (no locks needed)", async () => {
  await withTempDir(async (tempDir) => {
    await Deno.mkdir(`${tempDir}/f/folder`, { recursive: true });

    await Deno.writeTextFile(
      `${tempDir}/f/folder/my_script.sh`,
      `#!/bin/bash\necho hello`,
    );
    await Deno.writeTextFile(
      `${tempDir}/f/folder/my_script.script.yaml`,
      `summary: ""\ndescription: ""\nlock: ""\nkind: script\nschema:\n  $schema: "https://json-schema.org/draft/2020-12/schema"\n  type: object\n  properties: {}\n  required: []\n`,
    );

    const issues = await checkMissingLocks({} as any, tempDir);
    assertEquals(issues.length, 0);
  });
});

Deno.test("locks-required: skips SQL scripts (no locks needed)", async () => {
  await withTempDir(async (tempDir) => {
    await Deno.mkdir(`${tempDir}/f/folder`, { recursive: true });

    await Deno.writeTextFile(
      `${tempDir}/f/folder/my_query.pg.sql`,
      `SELECT 1;`,
    );
    await Deno.writeTextFile(
      `${tempDir}/f/folder/my_query.script.yaml`,
      `summary: ""\ndescription: ""\nlock: ""\nkind: script\nschema:\n  $schema: "https://json-schema.org/draft/2020-12/schema"\n  type: object\n  properties: {}\n  required: []\n`,
    );

    const issues = await checkMissingLocks({} as any, tempDir);
    assertEquals(issues.length, 0);
  });
});

Deno.test("locks-required: checks bun typescript scripts", async () => {
  await withTempDir(async (tempDir) => {
    await Deno.mkdir(`${tempDir}/f/folder`, { recursive: true });

    await Deno.writeTextFile(
      `${tempDir}/f/folder/my_script.bun.ts`,
      `export async function main() { return "hello"; }`,
    );
    await Deno.writeTextFile(
      `${tempDir}/f/folder/my_script.script.yaml`,
      `summary: ""\ndescription: ""\nlock: ""\nkind: script\nschema:\n  $schema: "https://json-schema.org/draft/2020-12/schema"\n  type: object\n  properties: {}\n  required: []\n`,
    );

    const issues = await checkMissingLocks({} as any, tempDir);
    assertEquals(issues.length, 1);
    assert(issues[0].errors[0].includes("bun"));
  });
});

Deno.test("locks-required: checks deno typescript scripts", async () => {
  await withTempDir(async (tempDir) => {
    await Deno.mkdir(`${tempDir}/f/folder`, { recursive: true });

    await Deno.writeTextFile(
      `${tempDir}/f/folder/my_script.deno.ts`,
      `export async function main() { return "hello"; }`,
    );
    await Deno.writeTextFile(
      `${tempDir}/f/folder/my_script.script.yaml`,
      `summary: ""\ndescription: ""\nlock: ""\nkind: script\nschema:\n  $schema: "https://json-schema.org/draft/2020-12/schema"\n  type: object\n  properties: {}\n  required: []\n`,
    );

    const issues = await checkMissingLocks({} as any, tempDir);
    assertEquals(issues.length, 1);
    assert(issues[0].errors[0].includes("deno"));
  });
});

// --- Flow inline script tests ---

Deno.test("locks-required: fails for flow with unlocked inline python script", async () => {
  await withTempDir(async (tempDir) => {
    await Deno.mkdir(`${tempDir}/f/my_flow.flow`, { recursive: true });

    await Deno.writeTextFile(
      `${tempDir}/f/my_flow.flow/inline_script_0.inline_script.py`,
      `def main(): pass`,
    );
    await Deno.writeTextFile(
      `${tempDir}/f/my_flow.flow/flow.yaml`,
      `summary: My flow
value:
  modules:
    - id: a
      value:
        type: rawscript
        language: python3
        content: "!inline inline_script_0.inline_script.py"
        lock: ""
`,
    );

    const issues = await checkMissingLocks({} as any, tempDir);
    assertEquals(issues.length, 1);
    assertEquals(issues[0].target, "flow_inline_script");
    assert(issues[0].errors[0].includes("python3"));
    assert(issues[0].errors[0].includes("'a'"));
  });
});

Deno.test("locks-required: passes for flow with locked inline python script", async () => {
  await withTempDir(async (tempDir) => {
    await Deno.mkdir(`${tempDir}/f/my_flow.flow`, { recursive: true });

    await Deno.writeTextFile(
      `${tempDir}/f/my_flow.flow/inline_script_0.inline_script.py`,
      `import pandas\ndef main(): pass`,
    );
    await Deno.writeTextFile(
      `${tempDir}/f/my_flow.flow/inline_script_0.inline_script.lock`,
      `pandas==2.0.0\n`,
    );
    await Deno.writeTextFile(
      `${tempDir}/f/my_flow.flow/flow.yaml`,
      `summary: My flow
value:
  modules:
    - id: a
      value:
        type: rawscript
        language: python3
        content: "!inline inline_script_0.inline_script.py"
        lock: "!inline inline_script_0.inline_script.lock"
`,
    );

    const issues = await checkMissingLocks({} as any, tempDir);
    assertEquals(issues.length, 0);
  });
});

Deno.test("locks-required: skips flow inline bash scripts", async () => {
  await withTempDir(async (tempDir) => {
    await Deno.mkdir(`${tempDir}/f/my_flow.flow`, { recursive: true });

    await Deno.writeTextFile(
      `${tempDir}/f/my_flow.flow/flow.yaml`,
      `summary: My flow
value:
  modules:
    - id: a
      value:
        type: rawscript
        language: bash
        content: "echo hello"
        lock: ""
`,
    );

    const issues = await checkMissingLocks({} as any, tempDir);
    assertEquals(issues.length, 0);
  });
});

Deno.test("locks-required: checks nested flow modules (forloopflow)", async () => {
  await withTempDir(async (tempDir) => {
    await Deno.mkdir(`${tempDir}/f/my_flow.flow`, { recursive: true });

    await Deno.writeTextFile(
      `${tempDir}/f/my_flow.flow/flow.yaml`,
      `summary: My flow
value:
  modules:
    - id: loop
      value:
        type: forloopflow
        modules:
          - id: inner
            value:
              type: rawscript
              language: python3
              content: "def main(): pass"
`,
    );

    const issues = await checkMissingLocks({} as any, tempDir);
    assertEquals(issues.length, 1);
    assert(issues[0].errors[0].includes("'inner'"));
  });
});

Deno.test("locks-required: checks nested flow modules (branchone)", async () => {
  await withTempDir(async (tempDir) => {
    await Deno.mkdir(`${tempDir}/f/my_flow.flow`, { recursive: true });

    await Deno.writeTextFile(
      `${tempDir}/f/my_flow.flow/flow.yaml`,
      `summary: My flow
value:
  modules:
    - id: branch
      value:
        type: branchone
        branches:
          - modules:
              - id: branch_script
                value:
                  type: rawscript
                  language: bun
                  content: "export async function main() {}"
        default:
          - id: default_script
            value:
              type: rawscript
              language: python3
              content: "def main(): pass"
`,
    );

    const issues = await checkMissingLocks({} as any, tempDir);
    assertEquals(issues.length, 2);
    const ids = issues.map((i) => i.errors[0]);
    assert(ids.some((e) => e.includes("'branch_script'")));
    assert(ids.some((e) => e.includes("'default_script'")));
  });
});

// --- Integration with runLint ---

Deno.test("locks-required: runLint includes lock issues when flag is set", async () => {
  await withTempDir(async (tempDir) => {
    await Deno.mkdir(`${tempDir}/f/folder`, { recursive: true });

    await Deno.writeTextFile(
      `${tempDir}/f/folder/my_script.py`,
      `def main(): pass`,
    );
    await Deno.writeTextFile(
      `${tempDir}/f/folder/my_script.script.yaml`,
      `summary: ""\ndescription: ""\nlock: ""\nkind: script\nschema:\n  $schema: "https://json-schema.org/draft/2020-12/schema"\n  type: object\n  properties: {}\n  required: []\n`,
    );

    const report = await runLint({ locksRequired: true } as any, tempDir);
    assertEquals(report.success, false);
    assertEquals(report.exitCode, 1);
    assert(report.issues.some((i) => i.target === "script"));
  });
});

Deno.test("locks-required: runLint skips lock check when flag is not set", async () => {
  await withTempDir(async (tempDir) => {
    await Deno.mkdir(`${tempDir}/f/folder`, { recursive: true });

    await Deno.writeTextFile(
      `${tempDir}/f/folder/my_script.py`,
      `def main(): pass`,
    );
    await Deno.writeTextFile(
      `${tempDir}/f/folder/my_script.script.yaml`,
      `summary: ""\ndescription: ""\nlock: ""\nkind: script\nschema:\n  $schema: "https://json-schema.org/draft/2020-12/schema"\n  type: object\n  properties: {}\n  required: []\n`,
    );

    const report = await runLint({} as any, tempDir);
    assertEquals(report.success, true);
    assertEquals(report.exitCode, 0);
    assertEquals(report.issues.length, 0);
  });
});

// --- Multiple scripts ---

Deno.test("locks-required: reports multiple missing locks", async () => {
  await withTempDir(async (tempDir) => {
    await Deno.mkdir(`${tempDir}/f/folder`, { recursive: true });

    // Python script without lock
    await Deno.writeTextFile(
      `${tempDir}/f/folder/script1.py`,
      `def main(): pass`,
    );
    await Deno.writeTextFile(
      `${tempDir}/f/folder/script1.script.yaml`,
      `summary: ""\ndescription: ""\nlock: ""\nkind: script\nschema:\n  $schema: "https://json-schema.org/draft/2020-12/schema"\n  type: object\n  properties: {}\n  required: []\n`,
    );

    // Go script without lock
    await Deno.writeTextFile(
      `${tempDir}/f/folder/script2.go`,
      `package main\nfunc main() {}`,
    );
    await Deno.writeTextFile(
      `${tempDir}/f/folder/script2.script.yaml`,
      `summary: ""\ndescription: ""\nlock: ""\nkind: script\nschema:\n  $schema: "https://json-schema.org/draft/2020-12/schema"\n  type: object\n  properties: {}\n  required: []\n`,
    );

    // Bash script (should pass - no lock needed)
    await Deno.writeTextFile(
      `${tempDir}/f/folder/script3.sh`,
      `#!/bin/bash\necho ok`,
    );
    await Deno.writeTextFile(
      `${tempDir}/f/folder/script3.script.yaml`,
      `summary: ""\ndescription: ""\nlock: ""\nkind: script\nschema:\n  $schema: "https://json-schema.org/draft/2020-12/schema"\n  type: object\n  properties: {}\n  required: []\n`,
    );

    const issues = await checkMissingLocks({} as any, tempDir);
    assertEquals(issues.length, 2);
    assert(issues.every((i) => i.target === "script"));
  });
});

// --- Normal app inline script tests ---

Deno.test("locks-required: fails for app with unlocked inline python script", async () => {
  await withTempDir(async (tempDir) => {
    await Deno.mkdir(`${tempDir}/f/my_app.app`, { recursive: true });

    await Deno.writeTextFile(
      `${tempDir}/f/my_app.app/app.yaml`,
      `summary: My app
value:
  grid:
    - data:
        inlineScript:
          content: "def main(): pass"
          language: python3
          lock: ""
`,
    );

    const issues = await checkMissingLocks({} as any, tempDir);
    assertEquals(issues.length, 1);
    assertEquals(issues[0].target, "app_inline_script");
    assert(issues[0].errors[0].includes("python3"));
  });
});

Deno.test("locks-required: passes for app with locked inline python script", async () => {
  await withTempDir(async (tempDir) => {
    await Deno.mkdir(`${tempDir}/f/my_app.app`, { recursive: true });

    await Deno.writeTextFile(
      `${tempDir}/f/my_app.app/inline_script_0.inline_script.lock`,
      `pandas==2.0.0\n`,
    );
    await Deno.writeTextFile(
      `${tempDir}/f/my_app.app/app.yaml`,
      `summary: My app
value:
  grid:
    - data:
        inlineScript:
          content: "import pandas"
          language: python3
          lock: "!inline inline_script_0.inline_script.lock"
`,
    );

    const issues = await checkMissingLocks({} as any, tempDir);
    assertEquals(issues.length, 0);
  });
});

Deno.test("locks-required: skips app inline bash scripts", async () => {
  await withTempDir(async (tempDir) => {
    await Deno.mkdir(`${tempDir}/f/my_app.app`, { recursive: true });

    await Deno.writeTextFile(
      `${tempDir}/f/my_app.app/app.yaml`,
      `summary: My app
value:
  grid:
    - data:
        inlineScript:
          content: "echo hello"
          language: bash
          lock: ""
`,
    );

    const issues = await checkMissingLocks({} as any, tempDir);
    assertEquals(issues.length, 0);
  });
});

Deno.test("locks-required: finds deeply nested app inline scripts", async () => {
  await withTempDir(async (tempDir) => {
    await Deno.mkdir(`${tempDir}/f/my_app.app`, { recursive: true });

    await Deno.writeTextFile(
      `${tempDir}/f/my_app.app/app.yaml`,
      `summary: My app
value:
  grid:
    - components:
        - nested:
            deeper:
              inlineScript:
                content: "export async function main() {}"
                language: bun
                lock: ""
`,
    );

    const issues = await checkMissingLocks({} as any, tempDir);
    assertEquals(issues.length, 1);
    assertEquals(issues[0].target, "app_inline_script");
    assert(issues[0].errors[0].includes("bun"));
  });
});

// --- Raw app backend script tests ---

Deno.test("locks-required: fails for raw app with unlocked backend python script", async () => {
  await withTempDir(async (tempDir) => {
    await Deno.mkdir(`${tempDir}/f/my_app.raw_app/backend`, { recursive: true });

    await Deno.writeTextFile(
      `${tempDir}/f/my_app.raw_app/raw_app.yaml`,
      `summary: My raw app
`,
    );
    await Deno.writeTextFile(
      `${tempDir}/f/my_app.raw_app/backend/get_data.yaml`,
      `type: inline
`,
    );
    await Deno.writeTextFile(
      `${tempDir}/f/my_app.raw_app/backend/get_data.py`,
      `def main(): pass`,
    );

    const issues = await checkMissingLocks({} as any, tempDir);
    assertEquals(issues.length, 1);
    assertEquals(issues[0].target, "raw_app_inline_script");
    assert(issues[0].errors[0].includes("python3"));
    assert(issues[0].errors[0].includes("get_data"));
  });
});

Deno.test("locks-required: passes for raw app with locked backend python script", async () => {
  await withTempDir(async (tempDir) => {
    await Deno.mkdir(`${tempDir}/f/my_app.raw_app/backend`, { recursive: true });

    await Deno.writeTextFile(
      `${tempDir}/f/my_app.raw_app/raw_app.yaml`,
      `summary: My raw app
`,
    );
    await Deno.writeTextFile(
      `${tempDir}/f/my_app.raw_app/backend/get_data.yaml`,
      `type: inline
`,
    );
    await Deno.writeTextFile(
      `${tempDir}/f/my_app.raw_app/backend/get_data.py`,
      `import pandas\ndef main(): pass`,
    );
    await Deno.writeTextFile(
      `${tempDir}/f/my_app.raw_app/backend/get_data.lock`,
      `pandas==2.0.0\n`,
    );

    const issues = await checkMissingLocks({} as any, tempDir);
    assertEquals(issues.length, 0);
  });
});

Deno.test("locks-required: raw app auto-detects code files without YAML config", async () => {
  await withTempDir(async (tempDir) => {
    await Deno.mkdir(`${tempDir}/f/my_app.raw_app/backend`, { recursive: true });

    await Deno.writeTextFile(
      `${tempDir}/f/my_app.raw_app/raw_app.yaml`,
      `summary: My raw app
`,
    );
    // No .yaml config, just a code file
    await Deno.writeTextFile(
      `${tempDir}/f/my_app.raw_app/backend/fetch_users.bun.ts`,
      `export async function main() { return []; }`,
    );

    const issues = await checkMissingLocks({} as any, tempDir);
    assertEquals(issues.length, 1);
    assertEquals(issues[0].target, "raw_app_inline_script");
    assert(issues[0].errors[0].includes("bun"));
    assert(issues[0].errors[0].includes("fetch_users"));
  });
});

Deno.test("locks-required: skips raw app bash backend scripts", async () => {
  await withTempDir(async (tempDir) => {
    await Deno.mkdir(`${tempDir}/f/my_app.raw_app/backend`, { recursive: true });

    await Deno.writeTextFile(
      `${tempDir}/f/my_app.raw_app/raw_app.yaml`,
      `summary: My raw app
`,
    );
    await Deno.writeTextFile(
      `${tempDir}/f/my_app.raw_app/backend/cleanup.yaml`,
      `type: inline
`,
    );
    await Deno.writeTextFile(
      `${tempDir}/f/my_app.raw_app/backend/cleanup.sh`,
      `#!/bin/bash\necho done`,
    );

    const issues = await checkMissingLocks({} as any, tempDir);
    assertEquals(issues.length, 0);
  });
});
