import {
  assert,
  assertEquals,
  assertStringIncludes,
} from "https://deno.land/std@0.224.0/assert/mod.ts";
import {
  formatValidationError,
  runLint,
} from "../src/commands/lint/lint.ts";

async function withTempDir(
  fn: (tempDir: string) => Promise<void>,
): Promise<void> {
  const tempDir = await Deno.makeTempDir({ prefix: "wmill_lint_test_" });
  const originalCwd = Deno.cwd();
  try {
    Deno.chdir(tempDir);
    await fn(tempDir);
  } finally {
    Deno.chdir(originalCwd);
    await Deno.remove(tempDir, { recursive: true });
  }
}

Deno.test("lint: validates flow, schedule, and trigger yaml files", async () => {
  await withTempDir(async (tempDir) => {
    await Deno.mkdir(`${tempDir}/f/my_flow.flow`, { recursive: true });
    await Deno.writeTextFile(
      `${tempDir}/f/my_flow.flow/flow.yaml`,
      `summary: My flow
value:
  modules: []
`,
    );

    await Deno.mkdir(`${tempDir}/f/jobs`, { recursive: true });
    await Deno.writeTextFile(
      `${tempDir}/f/jobs/daily.schedule.yaml`,
      `schedule: "0 0 12 * * *"
timezone: "UTC"
enabled: true
script_path: "f/jobs/daily_sync"
is_flow: false
`,
    );

    await Deno.mkdir(`${tempDir}/f/triggers`, { recursive: true });
    await Deno.writeTextFile(
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
    );

    const report = await runLint({} as any, tempDir);

    assertEquals(report.exitCode, 0);
    assertEquals(report.validatedFiles, 3);
    assertEquals(report.validFiles, 3);
    assertEquals(report.invalidFiles, 0);
    assertEquals(report.warnings.length, 0);
  });
});

Deno.test("lint: returns errors for invalid schedule documents", async () => {
  await withTempDir(async (tempDir) => {
    await Deno.mkdir(`${tempDir}/f/jobs`, { recursive: true });
    await Deno.writeTextFile(
      `${tempDir}/f/jobs/broken.schedule.yaml`,
      `timezone: "UTC"
enabled: true
script_path: "f/jobs/broken"
is_flow: false
`,
    );

    const report = await runLint({} as any, tempDir);

    assertEquals(report.exitCode, 1);
    assertEquals(report.validatedFiles, 1);
    assertEquals(report.invalidFiles, 1);
    assertEquals(report.issues[0].path, "f/jobs/broken.schedule.yaml");
    assert(
      report.issues[0].errors.some((message) =>
        message.includes("missing required property 'schedule'")
      ),
    );
  });
});

Deno.test("lint: warns and skips unsupported trigger schemas", async () => {
  await withTempDir(async (tempDir) => {
    await Deno.mkdir(`${tempDir}/f/triggers`, { recursive: true });
    await Deno.writeTextFile(
      `${tempDir}/f/triggers/mail.email_trigger.yaml`,
      `path: "f/triggers/mail"
`,
    );
    await Deno.writeTextFile(
      `${tempDir}/f/triggers/webhook.script.123.nextcloud_native_trigger.yaml`,
      `path: "f/triggers/native"
`,
    );

    const report = await runLint({} as any, tempDir);

    assertEquals(report.exitCode, 0);
    assertEquals(report.validatedFiles, 0);
    assertEquals(report.skippedUnsupportedFiles, 2);
    assertEquals(report.warnings.length, 2);
    assertStringIncludes(
      report.warnings[0].message,
      "Unsupported trigger schema",
    );

    const failOnWarnReport = await runLint(
      { failOnWarn: true } as any,
      tempDir,
    );
    assertEquals(failOnWarnReport.exitCode, 1);
  });
});

Deno.test("lint: uses wmill.yaml include filters for file discovery", async () => {
  await withTempDir(async (tempDir) => {
    await Deno.writeTextFile(
      `${tempDir}/wmill.yaml`,
      `defaultTs: bun
includes:
  - "f/allowed/**"
excludes: []
`,
    );

    await Deno.mkdir(`${tempDir}/f/allowed`, { recursive: true });
    await Deno.writeTextFile(
      `${tempDir}/f/allowed/ok.schedule.yaml`,
      `schedule: "0 0 12 * * *"
timezone: "UTC"
enabled: true
script_path: "f/jobs/ok"
is_flow: false
`,
    );

    await Deno.mkdir(`${tempDir}/f/blocked`, { recursive: true });
    await Deno.writeTextFile(
      `${tempDir}/f/blocked/bad.schedule.yaml`,
      `timezone: "UTC"
enabled: true
script_path: "f/jobs/bad"
is_flow: false
`,
    );

    const report = await runLint({} as any, tempDir);

    assertEquals(report.exitCode, 0);
    assertEquals(report.validatedFiles, 1);
    assertEquals(report.validFiles, 1);
    assertEquals(report.invalidFiles, 0);
    assertEquals(report.issues.length, 0);
  });
});

// --- formatValidationError unit tests ---

Deno.test("formatValidationError: required keyword", () => {
  assertEquals(
    formatValidationError({
      instancePath: "/value",
      keyword: "required",
      message: "must have required property 'modules'",
      params: { missingProperty: "modules" },
    }),
    "/value missing required property 'modules'",
  );
});

Deno.test("formatValidationError: additionalProperties keyword", () => {
  assertEquals(
    formatValidationError({
      instancePath: "/value",
      keyword: "additionalProperties",
      message: "must NOT have additional properties",
      params: { additionalProperty: "typo_field" },
    }),
    "/value has unknown property 'typo_field'",
  );
});

Deno.test("formatValidationError: enum keyword filters null values", () => {
  assertEquals(
    formatValidationError({
      instancePath: "/http_method",
      keyword: "enum",
      message: "must be equal to one of the allowed values",
      params: { allowedValues: [null, "get", "post", "put"] },
    }),
    "/http_method must be one of: 'get', 'post', 'put'",
  );
});

Deno.test("formatValidationError: falls back to message", () => {
  assertEquals(
    formatValidationError({
      instancePath: "/timeout",
      keyword: "type",
      message: "must be integer",
    }),
    "/timeout must be integer",
  );
});

Deno.test("formatValidationError: uses / for empty instancePath", () => {
  assertEquals(
    formatValidationError({
      instancePath: "",
      keyword: "required",
      message: "must have required property 'summary'",
      params: { missingProperty: "summary" },
    }),
    "/ missing required property 'summary'",
  );
});

Deno.test("formatValidationError: generic fallback when no message", () => {
  assertEquals(
    formatValidationError({ instancePath: "/field", keyword: "custom" }),
    "/field validation error",
  );
});

// --- runLint integration tests ---

Deno.test("lint: throws for non-existent directory", async () => {
  let threw = false;
  try {
    await runLint({} as any, "/tmp/wmill_lint_nonexistent_" + Date.now());
  } catch (e) {
    threw = true;
    assertStringIncludes((e as Error).message, "Directory not found");
  }
  assert(threw, "Expected runLint to throw for non-existent directory");
});

Deno.test("lint: json-shaped report contains all fields", async () => {
  await withTempDir(async (tempDir) => {
    await Deno.mkdir(`${tempDir}/f/jobs`, { recursive: true });
    await Deno.writeTextFile(
      `${tempDir}/f/jobs/ok.schedule.yaml`,
      `schedule: "0 0 * * *"
timezone: "UTC"
enabled: true
script_path: "f/jobs/ok"
is_flow: false
`,
    );

    const report = await runLint({ json: true } as any, tempDir);

    // Verify the report object has the shape expected by --json output
    assertEquals(typeof report.scannedFiles, "number");
    assertEquals(typeof report.validatedFiles, "number");
    assertEquals(typeof report.validFiles, "number");
    assertEquals(typeof report.invalidFiles, "number");
    assertEquals(typeof report.skippedUnsupportedFiles, "number");
    assert(Array.isArray(report.warnings));
    assert(Array.isArray(report.issues));
    assertEquals(typeof report.success, "boolean");
    assertEquals(typeof report.exitCode, "number");

    // JSON.stringify should round-trip cleanly
    const json = JSON.parse(JSON.stringify(report));
    assertEquals(json.success, true);
    assertEquals(json.exitCode, 0);
  });
});

Deno.test("lint: --fail-on-warn with mixed valid and warning files", async () => {
  await withTempDir(async (tempDir) => {
    // A valid schedule
    await Deno.mkdir(`${tempDir}/f/jobs`, { recursive: true });
    await Deno.writeTextFile(
      `${tempDir}/f/jobs/ok.schedule.yaml`,
      `schedule: "0 0 * * *"
timezone: "UTC"
enabled: true
script_path: "f/jobs/ok"
is_flow: false
`,
    );

    // An unsupported trigger that produces a warning
    await Deno.mkdir(`${tempDir}/f/triggers`, { recursive: true });
    await Deno.writeTextFile(
      `${tempDir}/f/triggers/inbox.email_trigger.yaml`,
      `path: "f/triggers/inbox"
`,
    );

    // Without --fail-on-warn: passes
    const normalReport = await runLint({} as any, tempDir);
    assertEquals(normalReport.exitCode, 0);
    assertEquals(normalReport.success, true);
    assertEquals(normalReport.validFiles, 1);
    assertEquals(normalReport.warnings.length, 1);

    // With --fail-on-warn: fails due to warning
    const strictReport = await runLint({ failOnWarn: true } as any, tempDir);
    assertEquals(strictReport.exitCode, 1);
    assertEquals(strictReport.success, false);
    assertEquals(strictReport.validFiles, 1);
    assertEquals(strictReport.warnings.length, 1);
  });
});

Deno.test("lint: reports enum errors with allowed values for invalid trigger", async () => {
  await withTempDir(async (tempDir) => {
    await Deno.mkdir(`${tempDir}/f/triggers`, { recursive: true });
    await Deno.writeTextFile(
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
    );

    const report = await runLint({} as any, tempDir);

    assertEquals(report.invalidFiles, 1);
    assert(
      report.issues[0].errors.some((msg) => msg.includes("must be one of:")),
      `Expected 'must be one of' error but got: ${report.issues[0].errors}`,
    );
  });
});
