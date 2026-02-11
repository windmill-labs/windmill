import {
  assert,
  assertEquals,
  assertStringIncludes,
} from "https://deno.land/std@0.224.0/assert/mod.ts";
import { runLint } from "../src/commands/lint/lint.ts";

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
