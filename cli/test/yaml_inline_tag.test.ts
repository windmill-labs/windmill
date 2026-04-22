/**
 * Unit tests for custom !inline and !inline_fileset YAML tag handling.
 * These tests require no backend — they test YAML parsing logic.
 */

import { expect, test, describe } from "bun:test";
import { yamlParseContent } from "../src/utils/yaml.ts";
import { stringify as yamlStringify } from "yaml";

describe("YAML !inline tag resolution", () => {
  test("unquoted !inline resolves to string with prefix", () => {
    const result = yamlParseContent("test.yaml", "content: !inline get_users.ts");
    expect(result.content).toBe("!inline get_users.ts");
  });

  test("quoted !inline is preserved as-is", () => {
    const result = yamlParseContent("test.yaml", 'content: "!inline get_users.ts"');
    expect(result.content).toBe("!inline get_users.ts");
  });

  test("unquoted and quoted produce identical results", () => {
    const unquoted = yamlParseContent("test.yaml", "content: !inline script.ts");
    const quoted = yamlParseContent("test.yaml", 'content: "!inline script.ts"');
    expect(unquoted.content).toBe(quoted.content);
  });

  test("unquoted !inline_fileset resolves to string with prefix", () => {
    const result = yamlParseContent("test.yaml", "value: !inline_fileset my_resource.fileset");
    expect(result.value).toBe("!inline_fileset my_resource.fileset");
  });

  test("works within nested flow.yaml structure", () => {
    const yaml = `
value:
  modules:
    - id: a
      value:
        type: rawscript
        content: !inline get_users.ts
        language: bun
    - id: b
      value:
        type: rawscript
        content: !inline send_mail.ts
        language: bun`;
    const result = yamlParseContent("flow.yaml", yaml);
    expect(result.value.modules[0].value.content).toBe("!inline get_users.ts");
    expect(result.value.modules[1].value.content).toBe("!inline send_mail.ts");
  });

  test("round-trip: parse unquoted → stringify → parse preserves value", () => {
    const yaml = "content: !inline my_script.ts";
    const parsed = yamlParseContent("test.yaml", yaml);
    const serialized = yamlStringify(parsed);
    const reparsed = yamlParseContent("test.yaml", serialized);
    expect(reparsed.content).toBe("!inline my_script.ts");
  });
});
