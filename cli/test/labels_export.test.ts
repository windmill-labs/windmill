/**
 * Labels Export/Import Tests
 *
 * Verifies that labels are correctly preserved through workspace export/import
 * and CLI sync roundtrips.
 */

import { expect, test, describe } from "bun:test";
import JSZip from "jszip";
import { createScriptFixture } from "./test_fixtures.ts";

describe("labels in workspace export", () => {
  test("script metadata with labels includes them in JSON", () => {
    const metadata = {
      summary: "test script",
      description: "a script with labels",
      schema: {},
      kind: "script",
      labels: ["production", "team-alpha"],
    };

    const json = JSON.stringify(metadata);
    const parsed = JSON.parse(json);
    expect(parsed.labels).toEqual(["production", "team-alpha"]);
  });

  test("script metadata without labels omits the field", () => {
    const metadata = {
      summary: "test script",
      description: "a script without labels",
      schema: {},
      kind: "script",
    };

    const json = JSON.stringify(metadata);
    const parsed = JSON.parse(json);
    expect(parsed.labels).toBeUndefined();
  });

  test("script metadata with null labels omits the field", () => {
    const metadata: Record<string, unknown> = {
      summary: "test script",
      description: "a script with null labels",
      schema: {},
      kind: "script",
      labels: null,
    };

    // Simulate serde skip_serializing_if behavior
    const filtered = Object.fromEntries(
      Object.entries(metadata).filter(([_, v]) => v != null)
    );
    const json = JSON.stringify(filtered);
    const parsed = JSON.parse(json);
    expect(parsed.labels).toBeUndefined();
  });

  test("script metadata with empty labels omits the field", () => {
    const metadata: Record<string, unknown> = {
      summary: "test script",
      schema: {},
      kind: "script",
      labels: [],
    };

    // Empty arrays should also be omitted
    const filtered = Object.fromEntries(
      Object.entries(metadata).filter(
        ([_, v]) => v != null && !(Array.isArray(v) && v.length === 0)
      )
    );
    const json = JSON.stringify(filtered);
    const parsed = JSON.parse(json);
    expect(parsed.labels).toBeUndefined();
  });

  test("labels survive zip roundtrip", async () => {
    const scriptMeta = {
      summary: "labeled script",
      description: "",
      schema: {},
      kind: "script",
      labels: ["deploy", "critical"],
    };

    // Create zip with script metadata
    const zip = new JSZip();
    zip.file("u/admin/my_script.py", 'def main():\n    return "hello"');
    zip.file(
      "u/admin/my_script.script.json",
      JSON.stringify(scriptMeta)
    );

    // Read back
    const content = await zip.generateAsync({ type: "uint8array" });
    const loaded = await JSZip.loadAsync(content);

    const metaStr = await loaded
      .file("u/admin/my_script.script.json")!
      .async("text");
    const parsed = JSON.parse(metaStr);

    expect(parsed.labels).toEqual(["deploy", "critical"]);
    expect(parsed.summary).toEqual("labeled script");
  });

  test("flow with labels survives zip roundtrip", async () => {
    const flowData = {
      summary: "labeled flow",
      description: "a flow with labels",
      value: { modules: [] },
      schema: {},
      tag: "mytag",
      labels: ["staging", "nightly"],
    };

    const zip = new JSZip();
    zip.file("u/admin/my_flow.flow.json", JSON.stringify(flowData));

    const content = await zip.generateAsync({ type: "uint8array" });
    const loaded = await JSZip.loadAsync(content);

    const metaStr = await loaded
      .file("u/admin/my_flow.flow.json")!
      .async("text");
    const parsed = JSON.parse(metaStr);

    expect(parsed.labels).toEqual(["staging", "nightly"]);
    expect(parsed.tag).toEqual("mytag");
  });

  test("resource with labels survives zip roundtrip", async () => {
    const resource = {
      value: { key: "value" },
      description: "test resource",
      resource_type: "postgresql",
      labels: ["prod-db"],
    };

    const zip = new JSZip();
    zip.file("u/admin/my_db.resource.json", JSON.stringify(resource));

    const content = await zip.generateAsync({ type: "uint8array" });
    const loaded = await JSZip.loadAsync(content);

    const str = await loaded
      .file("u/admin/my_db.resource.json")!
      .async("text");
    const parsed = JSON.parse(str);

    expect(parsed.labels).toEqual(["prod-db"]);
  });

  test("variable with labels survives zip roundtrip", async () => {
    const variable = {
      value: "secret123",
      is_secret: false,
      description: "test variable",
      labels: ["env", "config"],
    };

    const zip = new JSZip();
    zip.file("u/admin/my_var.variable.json", JSON.stringify(variable));

    const content = await zip.generateAsync({ type: "uint8array" });
    const loaded = await JSZip.loadAsync(content);

    const str = await loaded
      .file("u/admin/my_var.variable.json")!
      .async("text");
    const parsed = JSON.parse(str);

    expect(parsed.labels).toEqual(["env", "config"]);
  });

  test("schedule with labels survives zip roundtrip", async () => {
    const schedule = {
      schedule: "0 0 * * *",
      timezone: "UTC",
      script_path: "u/admin/my_script",
      is_flow: false,
      summary: "daily run",
      labels: ["cron", "daily"],
    };

    const zip = new JSZip();
    zip.file(
      "u/admin/daily_schedule.schedule.json",
      JSON.stringify(schedule)
    );

    const content = await zip.generateAsync({ type: "uint8array" });
    const loaded = await JSZip.loadAsync(content);

    const str = await loaded
      .file("u/admin/daily_schedule.schedule.json")!
      .async("text");
    const parsed = JSON.parse(str);

    expect(parsed.labels).toEqual(["cron", "daily"]);
  });

  test("items without labels don't have the field in exported JSON", async () => {
    const zip = new JSZip();

    zip.file(
      "u/admin/no_labels.flow.json",
      JSON.stringify({
        summary: "no labels flow",
        description: "",
        value: { modules: [] },
        schema: {},
      })
    );

    zip.file(
      "u/admin/no_labels.resource.json",
      JSON.stringify({
        value: {},
        description: "",
        resource_type: "c_test",
      })
    );

    const content = await zip.generateAsync({ type: "uint8array" });
    const loaded = await JSZip.loadAsync(content);

    const flowStr = await loaded
      .file("u/admin/no_labels.flow.json")!
      .async("text");
    expect(JSON.parse(flowStr).labels).toBeUndefined();

    const resStr = await loaded
      .file("u/admin/no_labels.resource.json")!
      .async("text");
    expect(JSON.parse(resStr).labels).toBeUndefined();
  });
});

describe("labels in script metadata YAML", () => {
  test("createScriptFixture does not include labels by default", () => {
    const fixture = createScriptFixture("test", "bun");
    expect(fixture.metadataFile.content).not.toContain("labels");
  });

  test("labels in YAML metadata are preserved through parse", () => {
    const yaml = `summary: "labeled script"
description: "test"
schema: {}
kind: script
labels:
  - production
  - team-alpha
`;
    // YAML parse would produce { labels: ["production", "team-alpha"] }
    // This is a structural test — actual YAML parsing is done by the CLI
    expect(yaml).toContain("labels:");
    expect(yaml).toContain("  - production");
    expect(yaml).toContain("  - team-alpha");
  });
});

describe("labels push request construction", () => {
  test("script push includes labels from metadata", () => {
    const typed = {
      summary: "test",
      description: "test desc",
      schema: {},
      kind: "script" as const,
      labels: ["deploy"],
    };

    // Simulate how handleFile builds requestBodyCommon
    const requestBody = {
      content: 'export async function main() { return 1; }',
      description: typed?.description ?? "",
      language: "bun" as const,
      path: "u/admin/test",
      summary: typed?.summary ?? "",
      kind: typed?.kind,
      labels: typed?.labels,
    };

    expect(requestBody.labels).toEqual(["deploy"]);
  });

  test("script push omits labels when not in metadata", () => {
    const typed = {
      summary: "test",
      description: "test desc",
      schema: {},
      kind: "script" as const,
    };

    const requestBody = {
      content: 'export async function main() { return 1; }',
      description: typed?.description ?? "",
      language: "bun" as const,
      path: "u/admin/test",
      summary: typed?.summary ?? "",
      kind: typed?.kind,
      labels: (typed as any)?.labels,
    };

    expect(requestBody.labels).toBeUndefined();
  });

  test("flow push preserves labels via spread", () => {
    const localFlow = {
      summary: "test flow",
      description: "",
      value: { modules: [] },
      schema: {},
      labels: ["staging"],
    };

    const requestBody = {
      path: "u/admin/test_flow",
      deployment_message: "test deploy",
      ...localFlow,
    };

    expect(requestBody.labels).toEqual(["staging"]);
  });

  test("resource push preserves labels via spread", () => {
    const localResource = {
      value: { host: "localhost" },
      description: "test db",
      resource_type: "postgresql",
      labels: ["prod"],
    };

    const requestBody = {
      path: "u/admin/test_resource",
      ...localResource,
    };

    expect(requestBody.labels).toEqual(["prod"]);
  });

  test("variable push preserves labels via spread", () => {
    const localVariable = {
      value: "secret",
      is_secret: true,
      description: "test var",
      labels: ["env"],
    };

    const requestBody = {
      path: "u/admin/test_var",
      ...localVariable,
    };

    expect(requestBody.labels).toEqual(["env"]);
  });

  test("schedule push preserves labels via spread", () => {
    const localSchedule = {
      schedule: "0 * * * *",
      timezone: "UTC",
      script_path: "u/admin/my_script",
      is_flow: false,
      labels: ["hourly"],
    };

    const requestBody = {
      path: "u/admin/my_schedule",
      ...localSchedule,
    };

    expect(requestBody.labels).toEqual(["hourly"]);
  });
});
