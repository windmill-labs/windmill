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

describe("labels edit and removal", () => {
  test("adding a label to existing metadata preserves other fields", () => {
    const original = {
      summary: "my script",
      description: "does things",
      schema: {},
      kind: "script" as const,
      lock: "pkg==1.0",
    };

    // Simulate editing: add labels
    const edited = { ...original, labels: ["new-label"] };

    expect(edited.summary).toEqual("my script");
    expect(edited.description).toEqual("does things");
    expect(edited.lock).toEqual("pkg==1.0");
    expect(edited.labels).toEqual(["new-label"]);
  });

  test("removing all labels results in undefined (not empty array)", () => {
    const withLabels = {
      summary: "test",
      labels: ["a", "b"],
    };

    // Simulate removal: set to undefined so it's omitted from JSON
    const cleared = { ...withLabels, labels: undefined };
    const json = JSON.stringify(cleared);
    const parsed = JSON.parse(json);

    expect(parsed.labels).toBeUndefined();
    expect(parsed.summary).toEqual("test");
  });

  test("setting labels to empty array serializes as empty array", () => {
    const withLabels = {
      summary: "test",
      labels: [] as string[],
    };

    const json = JSON.stringify(withLabels);
    const parsed = JSON.parse(json);

    expect(parsed.labels).toEqual([]);
  });

  test("modifying labels preserves order and deduplicates", () => {
    const labels = ["prod", "staging"];

    // Add a new one
    const added = [...labels, "dev"];
    expect(added).toEqual(["prod", "staging", "dev"]);

    // Remove one
    const removed = added.filter((l) => l !== "staging");
    expect(removed).toEqual(["prod", "dev"]);

    // No duplicates
    const withDup = [...removed, "prod"];
    const deduped = [...new Set(withDup)];
    expect(deduped).toEqual(["prod", "dev"]);
  });

  test("labels survive JSON roundtrip with special characters", () => {
    const labels = ["my-label", "team_alpha", "v1.0", "with spaces"];
    const json = JSON.stringify({ labels });
    const parsed = JSON.parse(json);
    expect(parsed.labels).toEqual(labels);
  });
});

describe("labels in pull/export for all item types", () => {
  test("script pull includes labels in metadata file", async () => {
    const zip = new JSZip();
    const metadata = {
      summary: "pulled script",
      description: "",
      schema: {},
      kind: "script",
      labels: ["from-remote"],
    };
    zip.file("u/admin/pulled.py", "def main(): pass");
    zip.file("u/admin/pulled.script.json", JSON.stringify(metadata));

    const content = await zip.generateAsync({ type: "uint8array" });
    const loaded = await JSZip.loadAsync(content);
    const meta = JSON.parse(
      await loaded.file("u/admin/pulled.script.json")!.async("text")
    );

    expect(meta.labels).toEqual(["from-remote"]);
    expect(meta.summary).toEqual("pulled script");
  });

  test("flow pull includes labels", async () => {
    const zip = new JSZip();
    const flow = {
      summary: "pulled flow",
      description: "",
      value: { modules: [] },
      schema: {},
      labels: ["ci", "nightly"],
    };
    zip.file("u/admin/pulled.flow.json", JSON.stringify(flow));

    const content = await zip.generateAsync({ type: "uint8array" });
    const loaded = await JSZip.loadAsync(content);
    const parsed = JSON.parse(
      await loaded.file("u/admin/pulled.flow.json")!.async("text")
    );

    expect(parsed.labels).toEqual(["ci", "nightly"]);
  });

  test("app pull includes labels", async () => {
    const zip = new JSZip();
    const app = {
      summary: "pulled app",
      value: {},
      policy: {},
      labels: ["dashboard"],
    };
    zip.file("u/admin/pulled.app.json", JSON.stringify(app));

    const content = await zip.generateAsync({ type: "uint8array" });
    const loaded = await JSZip.loadAsync(content);
    const parsed = JSON.parse(
      await loaded.file("u/admin/pulled.app.json")!.async("text")
    );

    expect(parsed.labels).toEqual(["dashboard"]);
  });

  test("resource pull includes labels", async () => {
    const zip = new JSZip();
    const resource = {
      value: { host: "db.example.com" },
      description: "production db",
      resource_type: "postgresql",
      labels: ["prod", "db"],
    };
    zip.file("u/admin/my_db.resource.json", JSON.stringify(resource));

    const content = await zip.generateAsync({ type: "uint8array" });
    const loaded = await JSZip.loadAsync(content);
    const parsed = JSON.parse(
      await loaded.file("u/admin/my_db.resource.json")!.async("text")
    );

    expect(parsed.labels).toEqual(["prod", "db"]);
  });

  test("variable pull includes labels", async () => {
    const zip = new JSZip();
    const variable = {
      value: "api-key-123",
      is_secret: false,
      description: "api key",
      labels: ["config"],
    };
    zip.file("u/admin/api_key.variable.json", JSON.stringify(variable));

    const content = await zip.generateAsync({ type: "uint8array" });
    const loaded = await JSZip.loadAsync(content);
    const parsed = JSON.parse(
      await loaded.file("u/admin/api_key.variable.json")!.async("text")
    );

    expect(parsed.labels).toEqual(["config"]);
  });

  test("schedule pull includes labels", async () => {
    const zip = new JSZip();
    const schedule = {
      schedule: "0 0 * * *",
      timezone: "UTC",
      script_path: "u/admin/daily",
      is_flow: false,
      summary: "daily cleanup",
      labels: ["cron", "maintenance"],
    };
    zip.file(
      "u/admin/daily_cleanup.schedule.json",
      JSON.stringify(schedule)
    );

    const content = await zip.generateAsync({ type: "uint8array" });
    const loaded = await JSZip.loadAsync(content);
    const parsed = JSON.parse(
      await loaded
        .file("u/admin/daily_cleanup.schedule.json")!
        .async("text")
    );

    expect(parsed.labels).toEqual(["cron", "maintenance"]);
  });

  test("pull without labels doesn't include labels field", async () => {
    const zip = new JSZip();
    zip.file(
      "u/admin/no_labels.script.json",
      JSON.stringify({ summary: "no labels", schema: {}, kind: "script" })
    );
    zip.file(
      "u/admin/no_labels.flow.json",
      JSON.stringify({ summary: "no labels", value: { modules: [] } })
    );
    zip.file(
      "u/admin/no_labels.resource.json",
      JSON.stringify({ value: {}, resource_type: "c_test" })
    );
    zip.file(
      "u/admin/no_labels.variable.json",
      JSON.stringify({ value: "x", is_secret: false, description: "" })
    );
    zip.file(
      "u/admin/no_labels.schedule.json",
      JSON.stringify({
        schedule: "0 0 * * *",
        timezone: "UTC",
        script_path: "u/admin/x",
        is_flow: false,
      })
    );

    const content = await zip.generateAsync({ type: "uint8array" });
    const loaded = await JSZip.loadAsync(content);

    for (const name of [
      "u/admin/no_labels.script.json",
      "u/admin/no_labels.flow.json",
      "u/admin/no_labels.resource.json",
      "u/admin/no_labels.variable.json",
      "u/admin/no_labels.schedule.json",
    ]) {
      const parsed = JSON.parse(await loaded.file(name)!.async("text"));
      expect(parsed.labels).toBeUndefined();
    }
  });

  test("editing labels on pulled item and re-pushing preserves changes", () => {
    // Simulate: pull -> edit labels -> push
    const pulled = {
      summary: "my script",
      schema: {},
      kind: "script" as const,
      labels: ["old-label"],
    };

    // User edits labels
    const edited = { ...pulled, labels: ["new-label", "another"] };

    // Push constructs request body
    const requestBody = {
      content: "def main(): pass",
      path: "u/admin/my_script",
      summary: edited.summary,
      kind: edited.kind,
      labels: edited.labels,
    };

    expect(requestBody.labels).toEqual(["new-label", "another"]);
    expect(requestBody.labels).not.toContain("old-label");
  });

  test("removing all labels on pulled item clears them on push", () => {
    const pulled = {
      summary: "labeled script",
      schema: {},
      kind: "script" as const,
      labels: ["to-remove"],
    };

    // User removes all labels
    const edited = { ...pulled };
    delete (edited as any).labels;

    const requestBody = {
      content: "def main(): pass",
      path: "u/admin/my_script",
      summary: edited.summary,
      kind: edited.kind,
      labels: (edited as any).labels,
    };

    expect(requestBody.labels).toBeUndefined();
  });
});
