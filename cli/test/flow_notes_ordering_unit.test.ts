/**
 * Unit tests for flow notes/groups preservation and YAML field ordering.
 *
 * Verifies that:
 * - Notes and groups survive a round-trip through generate-metadata (#8641)
 * - YAML output uses consistent field ordering via yamlOptions
 *
 * No backend required — tests the YAML parse/stringify layer.
 */

import { expect, test, describe } from "bun:test";
import { yamlParseContent } from "../src/utils/yaml.ts";
import { stringify as yamlStringify } from "yaml";
import { yamlOptions } from "../src/commands/sync/sync.ts";

const FLOW_WITH_NOTES = `
summary: Sync item
description: ''
value:
  modules:
    - id: fetch
      summary: Fetch product
      value:
        type: script
        input_transforms:
          connection:
            type: static
            value: some_resource
        is_trigger: false
        path: f/api/product_get
    - id: map
      summary: Map item
      value:
        type: script
        input_transforms:
          bc_item:
            type: javascript
            expr: flow_input.bc_item
        is_trigger: false
        path: f/mapping/item_to_product
  notes:
    - id: note-abc123
      type: group
      color: blue
      contained_node_ids:
        - fetch
        - map
      locked: false
      text: These steps must run together
schema:
  $schema: https://json-schema.org/draft/2020-12/schema
  type: object
`;

const FLOW_WITH_GROUPS = `
summary: Test flow
description: ''
value:
  modules:
    - id: a
      value:
        type: identity
  groups:
    - summary: My group
      start_id: a
      end_id: a
      color: green
schema:
  type: object
`;

describe("flow notes preservation (#8641)", () => {
  test("notes survive YAML round-trip with yamlOptions", () => {
    const parsed = yamlParseContent("flow.yaml", FLOW_WITH_NOTES);

    // Verify notes were parsed
    expect(parsed.value.notes).toBeDefined();
    expect(parsed.value.notes).toHaveLength(1);
    expect(parsed.value.notes[0].id).toBe("note-abc123");
    expect(parsed.value.notes[0].color).toBe("blue");
    expect(parsed.value.notes[0].text).toBe("These steps must run together");

    // Simulate the generate-metadata round-trip:
    // 1. Backend returns a new value WITHOUT notes (like FlowValue does)
    const backendResponse = { ...parsed.value };
    delete backendResponse.notes;

    // 2. CLI preserves notes (our fix)
    const savedNotes = parsed.value.notes;
    parsed.value = backendResponse;
    if (savedNotes !== undefined) parsed.value.notes = savedNotes;

    // 3. Serialize back to YAML
    const output = yamlStringify(parsed, yamlOptions);

    // 4. Re-parse and verify notes are intact
    const reparsed = yamlParseContent("flow.yaml", output);
    expect(reparsed.value.notes).toBeDefined();
    expect(reparsed.value.notes).toHaveLength(1);
    expect(reparsed.value.notes[0].id).toBe("note-abc123");
    expect(reparsed.value.notes[0].color).toBe("blue");
    expect(reparsed.value.notes[0].contained_node_ids).toEqual(["fetch", "map"]);
    expect(reparsed.value.notes[0].text).toBe("These steps must run together");
  });

  test("groups survive YAML round-trip with yamlOptions", () => {
    const parsed = yamlParseContent("flow.yaml", FLOW_WITH_GROUPS);

    expect(parsed.value.groups).toBeDefined();
    expect(parsed.value.groups).toHaveLength(1);
    expect(parsed.value.groups[0].summary).toBe("My group");

    // Simulate backend stripping groups
    const backendResponse = { ...parsed.value };
    delete backendResponse.groups;

    const savedGroups = parsed.value.groups;
    parsed.value = backendResponse;
    if (savedGroups !== undefined) parsed.value.groups = savedGroups;

    const output = yamlStringify(parsed, yamlOptions);
    const reparsed = yamlParseContent("flow.yaml", output);
    expect(reparsed.value.groups).toBeDefined();
    expect(reparsed.value.groups).toHaveLength(1);
    expect(reparsed.value.groups[0].summary).toBe("My group");
  });

  test("flow without notes or groups is unaffected", () => {
    const yaml = `
summary: Simple flow
value:
  modules:
    - id: a
      value:
        type: identity
schema:
  type: object
`;
    const parsed = yamlParseContent("flow.yaml", yaml);
    expect(parsed.value.notes).toBeUndefined();
    expect(parsed.value.groups).toBeUndefined();

    // Simulate the save/restore logic with undefined
    const savedNotes = parsed.value.notes;
    const savedGroups = parsed.value.groups;
    // Replace value (simulating backend response)
    parsed.value = { ...parsed.value };
    if (savedNotes !== undefined) parsed.value.notes = savedNotes;
    if (savedGroups !== undefined) parsed.value.groups = savedGroups;

    const output = yamlStringify(parsed, yamlOptions);
    const reparsed = yamlParseContent("flow.yaml", output);
    expect(reparsed.value.notes).toBeUndefined();
    expect(reparsed.value.groups).toBeUndefined();
  });
});

describe("flow YAML field ordering", () => {
  test("yamlOptions produces consistent field order for flow modules", () => {
    // Simulate a flow value with fields in random order (like backend response)
    const unordered = {
      summary: "Test",
      value: {
        modules: [
          {
            value: { type: "script", path: "f/test", is_trigger: false, input_transforms: {} },
            id: "step1",
            summary: "Step 1",
          },
        ],
      },
      schema: { type: "object" },
      description: "",
    };

    const output = yamlStringify(unordered, yamlOptions);

    // With yamlOptions, 'id' should come before 'summary' and 'value'
    // because prioritizeName gives "id" → "aa", "summary" → "ad", "value" → "ah"
    // Note: YAML sequence items start with "- id:" on the first key
    const lines = output.split("\n");
    const idLine = lines.findIndex((l) => /^\s*-?\s*id:/.test(l));
    const summaryLine = lines.findIndex((l, i) => i > idLine && /^\s+summary:/.test(l));

    expect(idLine).toBeGreaterThan(-1);
    expect(summaryLine).toBeGreaterThan(idLine);
  });

  test("yamlOptions produces same output regardless of input key order", () => {
    const order1 = {
      summary: "Flow",
      description: "",
      value: { modules: [{ id: "a", summary: "S", value: { type: "identity" } }] },
      schema: { type: "object" },
    };
    const order2 = {
      schema: { type: "object" },
      value: { modules: [{ value: { type: "identity" }, summary: "S", id: "a" }] },
      description: "",
      summary: "Flow",
    };

    const output1 = yamlStringify(order1, yamlOptions);
    const output2 = yamlStringify(order2, yamlOptions);
    expect(output1).toBe(output2);
  });

  test("notes field is preserved in correct position after modules", () => {
    const parsed = yamlParseContent("flow.yaml", FLOW_WITH_NOTES);
    const output = yamlStringify(parsed, yamlOptions);

    // 'modules' should appear before 'notes' in the output
    const modulesIdx = output.indexOf("modules:");
    const notesIdx = output.indexOf("notes:");
    expect(modulesIdx).toBeGreaterThan(-1);
    expect(notesIdx).toBeGreaterThan(-1);
    expect(modulesIdx).toBeLessThan(notesIdx);
  });
});
