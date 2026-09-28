import { expect, test } from "bun:test";
import { mkdtemp, rm, mkdir, writeFile } from "node:fs/promises";
import os from "node:os";
import * as path from "node:path";
import {
  formatValidationError,
  runLint,
} from "../src/commands/lint/lint.ts";

async function withTempDir(
  fn: (tempDir: string) => Promise<void>,
): Promise<void> {
  const tempDir = await mkdtemp(path.join(os.tmpdir(), "wmill_lint_test_"));
  const originalCwd = process.cwd();
  try {
    process.chdir(tempDir);
    await fn(tempDir);
  } finally {
    process.chdir(originalCwd);
    await rm(tempDir, { recursive: true });
  }
}

test("lint: validates flow, schedule, and trigger yaml files", async () => {
  await withTempDir(async (tempDir) => {
    await mkdir(`${tempDir}/f/my_flow.flow`, { recursive: true });
    await writeFile(
      `${tempDir}/f/my_flow.flow/flow.yaml`,
      `summary: My flow
value:
  modules: []
`,
      "utf-8"
    );

    await mkdir(`${tempDir}/f/jobs`, { recursive: true });
    await writeFile(
      `${tempDir}/f/jobs/daily.schedule.yaml`,
      `schedule: "0 0 12 * * *"
timezone: "UTC"
enabled: true
script_path: "f/jobs/daily_sync"
is_flow: false
`,
      "utf-8"
    );

    await mkdir(`${tempDir}/f/triggers`, { recursive: true });
    await writeFile(
      `${tempDir}/f/triggers/hook.http_trigger.yaml`,
      `script_path: "f/triggers/http_handler"
is_flow: false
route_path: "api/webhook"
request_type: "sync"
authentication_method: "none"
http_method: "post"
is_static_website: false
workspaced_route: false
wrap_body: false
raw_string: false
`,
      "utf-8"
    );

    await writeFile(
      `${tempDir}/f/triggers/inbox.email_trigger.yaml`,
      `script_path: "f/triggers/email_handler"
is_flow: false
local_part: "inbox"
`,
      "utf-8"
    );

    const report = await runLint({} as any, tempDir);

    expect(report.exitCode).toEqual(0);
    expect(report.validatedFiles).toEqual(4);
    expect(report.validFiles).toEqual(4);
    expect(report.invalidFiles).toEqual(0);
    expect(report.warnings.length).toEqual(0);
  });
});

// Guards against lint validating flows with a schema older than the checkout: a step
// linked to an `ai_agent` resource carries neither `tools` nor provider/output_type.
test("lint: accepts an aiagent step linked to an ai_agent resource", async () => {
  await withTempDir(async (tempDir) => {
    await mkdir(`${tempDir}/f/agents/support.flow`, { recursive: true });
    await writeFile(
      `${tempDir}/f/agents/support.flow/flow.yaml`,
      `summary: Support triage
value:
  modules:
    - id: a
      value:
        type: aiagent
        agent: f/agents/support_agent
        input_transforms:
          user_message:
            type: javascript
            expr: flow_input.question
`,
      "utf-8"
    );

    const report = await runLint({} as any, tempDir);

    expect(report.issues).toEqual([]);
    expect(report.exitCode).toEqual(0);
    expect(report.validFiles).toEqual(1);
  });
});

test("lint: returns errors for invalid schedule documents", async () => {
  await withTempDir(async (tempDir) => {
    await mkdir(`${tempDir}/f/jobs`, { recursive: true });
    await writeFile(
      `${tempDir}/f/jobs/broken.schedule.yaml`,
      `timezone: "UTC"
enabled: true
script_path: "f/jobs/broken"
is_flow: false
`,
      "utf-8"
    );

    const report = await runLint({} as any, tempDir);

    expect(report.exitCode).toEqual(1);
    expect(report.validatedFiles).toEqual(1);
    expect(report.invalidFiles).toEqual(1);
    expect(report.issues[0].path).toEqual("f/jobs/broken.schedule.yaml");
    expect(
      report.issues[0].errors.some((message) =>
        message.includes("missing required property 'schedule'")
      ),
    ).toBeTruthy();
  });
});

test("lint: warns and skips unsupported native trigger schemas", async () => {
  await withTempDir(async (tempDir) => {
    await mkdir(`${tempDir}/f/triggers`, { recursive: true });
    await writeFile(
      `${tempDir}/f/triggers/webhook.script.123.nextcloud_native_trigger.yaml`,
      `path: "f/triggers/native"
`,
      "utf-8"
    );

    const report = await runLint({} as any, tempDir);

    expect(report.exitCode).toEqual(0);
    expect(report.validatedFiles).toEqual(0);
    expect(report.skippedUnsupportedFiles).toEqual(1);
    expect(report.warnings.length).toEqual(1);
    expect(
      report.warnings[0].message,
    ).toContain("Unsupported trigger schema");

    const failOnWarnReport = await runLint(
      { failOnWarn: true } as any,
      tempDir,
    );
    expect(failOnWarnReport.exitCode).toEqual(1);
  });
});

test("lint: uses wmill.yaml include filters for file discovery", async () => {
  await withTempDir(async (tempDir) => {
    await writeFile(
      `${tempDir}/wmill.yaml`,
      `defaultTs: bun
includes:
  - "f/allowed/**"
excludes: []
`,
      "utf-8"
    );

    await mkdir(`${tempDir}/f/allowed`, { recursive: true });
    await writeFile(
      `${tempDir}/f/allowed/ok.schedule.yaml`,
      `schedule: "0 0 12 * * *"
timezone: "UTC"
enabled: true
script_path: "f/jobs/ok"
is_flow: false
`,
      "utf-8"
    );

    await mkdir(`${tempDir}/f/blocked`, { recursive: true });
    await writeFile(
      `${tempDir}/f/blocked/bad.schedule.yaml`,
      `timezone: "UTC"
enabled: true
script_path: "f/jobs/bad"
is_flow: false
`,
      "utf-8"
    );

    const report = await runLint({} as any, tempDir);

    expect(report.exitCode).toEqual(0);
    expect(report.validatedFiles).toEqual(1);
    expect(report.validFiles).toEqual(1);
    expect(report.invalidFiles).toEqual(0);
    expect(report.issues.length).toEqual(0);
  });
});

// --- formatValidationError unit tests ---

test("formatValidationError: required keyword", () => {
  expect(
    formatValidationError({
      instancePath: "/value",
      keyword: "required",
      message: "must have required property 'modules'",
      params: { missingProperty: "modules" },
    }),
  ).toEqual("/value missing required property 'modules'");
});

test("formatValidationError: additionalProperties keyword", () => {
  expect(
    formatValidationError({
      instancePath: "/value",
      keyword: "additionalProperties",
      message: "must NOT have additional properties",
      params: { additionalProperty: "typo_field" },
    }),
  ).toEqual("/value has unknown property 'typo_field'");
});

test("formatValidationError: enum keyword filters null values", () => {
  expect(
    formatValidationError({
      instancePath: "/http_method",
      keyword: "enum",
      message: "must be equal to one of the allowed values",
      params: { allowedValues: [null, "get", "post", "put"] },
    }),
  ).toEqual("/http_method must be one of: 'get', 'post', 'put'");
});

test("formatValidationError: falls back to message", () => {
  expect(
    formatValidationError({
      instancePath: "/timeout",
      keyword: "type",
      message: "must be integer",
    }),
  ).toEqual("/timeout must be integer");
});

test("formatValidationError: uses / for empty instancePath", () => {
  expect(
    formatValidationError({
      instancePath: "",
      keyword: "required",
      message: "must have required property 'summary'",
      params: { missingProperty: "summary" },
    }),
  ).toEqual("/ missing required property 'summary'");
});

test("formatValidationError: generic fallback when no message", () => {
  expect(
    formatValidationError({ instancePath: "/field", keyword: "custom" }),
  ).toEqual("/field validation error");
});

// --- runLint integration tests ---

test("lint: throws for non-existent directory", async () => {
  let threw = false;
  try {
    await runLint({} as any, "/tmp/wmill_lint_nonexistent_" + Date.now());
  } catch (e) {
    threw = true;
    expect((e as Error).message).toContain("Directory not found");
  }
  expect(threw).toBeTruthy();
});

test("lint: json-shaped report contains all fields", async () => {
  await withTempDir(async (tempDir) => {
    await mkdir(`${tempDir}/f/jobs`, { recursive: true });
    await writeFile(
      `${tempDir}/f/jobs/ok.schedule.yaml`,
      `schedule: "0 0 * * *"
timezone: "UTC"
enabled: true
script_path: "f/jobs/ok"
is_flow: false
`,
      "utf-8"
    );

    const report = await runLint({ json: true } as any, tempDir);

    // Verify the report object has the shape expected by --json output
    expect(typeof report.scannedFiles).toEqual("number");
    expect(typeof report.validatedFiles).toEqual("number");
    expect(typeof report.validFiles).toEqual("number");
    expect(typeof report.invalidFiles).toEqual("number");
    expect(typeof report.skippedUnsupportedFiles).toEqual("number");
    expect(Array.isArray(report.warnings)).toBeTruthy();
    expect(Array.isArray(report.issues)).toBeTruthy();
    expect(typeof report.success).toEqual("boolean");
    expect(typeof report.exitCode).toEqual("number");

    // JSON.stringify should round-trip cleanly
    const json = JSON.parse(JSON.stringify(report));
    expect(json.success).toEqual(true);
    expect(json.exitCode).toEqual(0);
  });
});

test("lint: --fail-on-warn with mixed valid and warning files", async () => {
  await withTempDir(async (tempDir) => {
    // A valid schedule
    await mkdir(`${tempDir}/f/jobs`, { recursive: true });
    await writeFile(
      `${tempDir}/f/jobs/ok.schedule.yaml`,
      `schedule: "0 0 * * *"
timezone: "UTC"
enabled: true
script_path: "f/jobs/ok"
is_flow: false
`,
      "utf-8"
    );

    // An unsupported native trigger that produces a warning
    await mkdir(`${tempDir}/f/triggers`, { recursive: true });
    await writeFile(
      `${tempDir}/f/triggers/webhook.script.123.nextcloud_native_trigger.yaml`,
      `path: "f/triggers/native"
`,
      "utf-8"
    );

    // Without --fail-on-warn: passes
    const normalReport = await runLint({} as any, tempDir);
    expect(normalReport.exitCode).toEqual(0);
    expect(normalReport.success).toEqual(true);
    expect(normalReport.validFiles).toEqual(1);
    expect(normalReport.warnings.length).toEqual(1);

    // With --fail-on-warn: fails due to warning
    const strictReport = await runLint({ failOnWarn: true } as any, tempDir);
    expect(strictReport.exitCode).toEqual(1);
    expect(strictReport.success).toEqual(false);
    expect(strictReport.validFiles).toEqual(1);
    expect(strictReport.warnings.length).toEqual(1);
  });
});

test("lint: reports enum errors with allowed values for invalid trigger", async () => {
  await withTempDir(async (tempDir) => {
    await mkdir(`${tempDir}/f/triggers`, { recursive: true });
    await writeFile(
      `${tempDir}/f/triggers/hook.http_trigger.yaml`,
      `script_path: "f/triggers/http_handler"
is_flow: false
route_path: "api/webhook"
authentication_method: "none"
http_method: "invalid_method"
is_static_website: false
workspaced_route: false
wrap_body: false
raw_string: false
`,
      "utf-8"
    );

    const report = await runLint({} as any, tempDir);

    expect(report.invalidFiles).toEqual(1);
    expect(
      report.issues[0].errors.some((msg) => msg.includes("must be one of:")),
    ).toBeTruthy();
  });
});

async function lintFlow(tempDir: string, value: string) {
  await mkdir(`${tempDir}/f/sem/check.flow`, { recursive: true });
  await writeFile(
    `${tempDir}/f/sem/check.flow/flow.yaml`,
    `summary: Semantic checks\nvalue:\n${value}`,
    "utf-8",
  );
  return await runLint({} as any, tempDir);
}

test("lint: rejects AI agent tool names the worker refuses", async () => {
  await withTempDir(async (tempDir) => {
    const report = await lintFlow(
      tempDir,
      `  modules:
    - id: agent
      value:
        type: aiagent
        agent: f/sem/support_agent
        input_transforms: {}
        tools:
          - id: t_spaced
            summary: Search docs
            value: { tool_type: flowmodule, type: script, path: f/sem/search, input_transforms: {} }
          - id: t_missing
            value: { tool_type: flowmodule, type: script, path: f/sem/search, input_transforms: {} }
          - id: t_first
            summary: get_user
            value: { tool_type: flowmodule, type: script, path: f/sem/get_user, input_transforms: {} }
          - id: t_dup
            summary: get_user
            value: { tool_type: flowmodule, type: script, path: f/sem/get_user, input_transforms: {} }
          - id: t_reserved
            summary: if
            value: { tool_type: flowmodule, type: script, path: f/sem/branch, input_transforms: {} }
          - id: t_mcp
            summary: ""
            value: { tool_type: mcp, resource_path: f/sem/mcp }
          - id: t_web
            summary: Web Search
            value: { tool_type: websearch }
`,
    );

    const errors = report.issues.flatMap((i) => i.errors);
    expect(errors).toEqual([
      "/value/modules/0/value/tools/0 tool name 'Search docs' may only contain letters, numbers and underscores (every run fails with 'Invalid tool name')",
      "/value/modules/0/value/tools/1 has no summary: a tool's summary is the name the agent calls it by",
      "/value/modules/0/value/tools/3 tool name 'get_user' is already used by tool 2 of this agent",
    ]);
    expect(report.warnings.map((w) => w.message)).toEqual([
      "/value/modules/0/value/tools/4 tool name 'if' is a reserved id; the flow editor refuses it",
    ]);
    expect(report.exitCode).toEqual(1);
  });
});

test("lint: rejects flow groups the editor cannot draw", async () => {
  await withTempDir(async (tempDir) => {
    const step = (id: string) =>
      `    - id: ${id}\n      value: { type: identity }\n`;
    const report = await lintFlow(
      tempDir,
      `  modules:
${step("a")}${step("b")}${step("c")}    - id: loop
      value:
        type: forloopflow
        iterator: { type: javascript, expr: "[1]" }
        skip_failures: false
        modules:
          - id: inner
            value: { type: identity }
  groups:
    - { start_id: c, end_id: a }
    - { start_id: a, end_id: inner }
    - { start_id: a, end_id: gone }
    - { start_id: a, end_id: b }
    - { start_id: b, end_id: c }
    - { start_id: a, end_id: b }
`,
    );

    expect(report.issues.flatMap((i) => i.errors)).toEqual([
      "/value/groups/0 starts at 'c', which comes after its end 'a'",
      "/value/groups/1 starts at 'a' and ends at 'inner', which are not in the same branch",
      "/value/groups/2 references 'gone', which is not a step of this flow",
      "/value/groups/5 spans the same steps as another group (a to b)",
      "/value/groups/4 partly overlaps /value/groups/3: groups may nest but not overlap",
    ]);
  });
});

test("lint: accepts nested groups and flags notes the editor renders wrong", async () => {
  await withTempDir(async (tempDir) => {
    const report = await lintFlow(
      tempDir,
      `  modules:
    - { id: a, value: { type: identity } }
    - { id: b, value: { type: identity } }
    - id: loop
      value:
        type: forloopflow
        iterator: { type: javascript, expr: "[1]" }
        skip_failures: false
        modules:
          - { id: x, value: { type: identity } }
          - { id: y, value: { type: identity } }
  groups:
    - { summary: Fetch, start_id: a, end_id: loop, color: blue }
    - { summary: Prep, start_id: a, end_id: b }
    - { summary: Body, start_id: x, end_id: y, color: "#ff0000" }
  notes:
    - { id: n1, text: Purpose, color: yellow, type: free, position: { x: -400, y: 0 }, size: { width: 275, height: 80 } }
    - { id: n2, text: Unplaced, color: yellow, type: free }
    - { id: n3, text: Old, color: gray, type: group, contained_node_ids: [a] }
`,
    );

    expect(report.issues).toEqual([]);
    expect(report.warnings.map((w) => w.message)).toEqual([
      "/value/groups/2 color '#ff0000' renders unstyled; use one of: yellow, blue, green, purple, pink, orange, red, cyan, lime, gray",
      "/value/notes/1 has no position or size, so the editor places it at the origin and cannot resize it",
      "/value/notes/2 is a 'group' note, which is deprecated: use value.groups instead",
    ]);
    expect(report.exitCode).toEqual(0);
  });
});
