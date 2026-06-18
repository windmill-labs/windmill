/**
 * Unit tests for wmill.yaml template generation, config reference, and JSON Schema.
 */

import { expect, test, describe } from "bun:test";
import { parse } from "yaml";
import Ajv from "ajv";
import {
  generateCommentedTemplate,
  generateJsonSchema,
  formatConfigReference,
  formatConfigReferenceJson,
  CONFIG_REFERENCE,
} from "../src/commands/init/template.ts";

// =============================================================================
// generateCommentedTemplate
// =============================================================================

describe("generateCommentedTemplate", () => {
  test("produces valid YAML that parses without errors", () => {
    const yaml = generateCommentedTemplate("main");
    const config = parse(yaml);
    expect(config).toBeDefined();
    expect(typeof config).toBe("object");
  });

  test("uses provided branch name in workspaces", () => {
    const config = parse(generateCommentedTemplate("my-feature"));
    expect(config.workspaces["my-feature"]).toBeDefined();
  });

  test("defaults to 'main' when no branch name given", () => {
    const config = parse(generateCommentedTemplate());
    expect(config.workspaces["main"]).toBeDefined();
  });

  test("quotes branch names with YAML-special characters", () => {
    const specialBranches = ["fix: something", "feat/my branch", "release#1"];
    for (const branch of specialBranches) {
      const yaml = generateCommentedTemplate(branch);
      const config = parse(yaml);
      expect(config.workspaces[branch]).toBeDefined();
    }
  });

  test("contains yaml-language-server schema directive", () => {
    const yaml = generateCommentedTemplate("main");
    expect(yaml.startsWith("# yaml-language-server: $schema=wmill.schema.json")).toBe(true);
  });

  test("includes all non-commented, non-skipped CONFIG_REFERENCE entries as active YAML keys", () => {
    const config = parse(generateCommentedTemplate("main"));
    for (const opt of CONFIG_REFERENCE) {
      if (!opt.commented && !opt.skipInTemplate) {
        expect(config).toHaveProperty(opt.name);
      }
    }
  });

  test("does not include commented or skipped entries as active YAML keys", () => {
    const config = parse(generateCommentedTemplate("main"));
    for (const opt of CONFIG_REFERENCE) {
      if (opt.commented || opt.skipInTemplate) {
        expect(config[opt.name]).toBeUndefined();
      }
    }
  });

  test("default values match expected defaults", () => {
    const config = parse(generateCommentedTemplate("main"));
    expect(config.defaultTs).toBe("bun");
    expect(config.skipSecrets).toBe(true);
    expect(config.nonDottedPaths).toBe(true);
    expect(config.codebases).toEqual([]);
    expect(config.excludes).toEqual([]);
    expect(config.includes).toEqual(["f/**"]);
  });
});

// =============================================================================
// generateJsonSchema
// =============================================================================

describe("generateJsonSchema", () => {
  const schema = generateJsonSchema();

  test("is a valid JSON Schema draft-07", () => {
    expect(schema.$schema).toBe("http://json-schema.org/draft-07/schema#");
    expect(schema.type).toBe("object");
    expect(schema.properties).toBeDefined();
  });

  test("validates the generated YAML template", () => {
    const config = parse(generateCommentedTemplate("main"));
    const ajv = new Ajv({ allErrors: true });
    const validate = ajv.compile(schema);
    expect(validate(config)).toBe(true);
  });

  test("rejects unknown keys", () => {
    const ajv = new Ajv({ allErrors: true });
    const validate = ajv.compile(schema);
    expect(validate({ unknownOption: true })).toBe(false);
  });

  test("rejects invalid enum values", () => {
    const ajv = new Ajv({ allErrors: true });
    const validate = ajv.compile(schema);
    expect(validate({ defaultTs: "python" })).toBe(false);
  });

  test("rejects wrong types", () => {
    const ajv = new Ajv({ allErrors: true });
    const validate = ajv.compile(schema);
    expect(validate({ skipSecrets: "yes" })).toBe(false);
  });

  test("includes codebases array schema with item properties", () => {
    expect(schema.properties.codebases.type).toBe("array");
    expect(schema.properties.codebases.items.properties.relative_path).toBeDefined();
    expect(schema.properties.codebases.items.required).toContain("relative_path");
  });

  test("includes workspaces with workspace config schema", () => {
    const wsSchema = schema.properties.workspaces.additionalProperties;
    expect(wsSchema.properties.gitBranch).toBeDefined();
    expect(wsSchema.properties.baseUrl).toBeDefined();
    expect(wsSchema.properties.workspaceId).toBeDefined();
    expect(wsSchema.properties.specificItems).toBeDefined();
    expect(wsSchema.properties.specificItems.properties.variables).toBeDefined();
  });

  test("includes gitBranches as deprecated alias for workspaces", () => {
    expect(schema.properties.gitBranches).toBeDefined();
    expect(schema.properties.gitBranches.additionalProperties).toEqual(
      schema.properties.workspaces.additionalProperties
    );
  });

  test("includes environments as deprecated alias for workspaces", () => {
    expect(schema.properties.environments).toBeDefined();
    expect(schema.properties.environments.additionalProperties).toEqual(
      schema.properties.workspaces.additionalProperties
    );
  });

  test("does not contain template-only keys in schema output", () => {
    const templateKeys = ["section", "sectionNote", "commented", "templateValue", "example", "inlineComment", "groupNote", "skipInTemplate"];
    const json = JSON.stringify(schema);
    for (const key of templateKeys) {
      expect(json).not.toContain(`"${key}"`);
    }
  });
});

// =============================================================================
// formatConfigReference
// =============================================================================

describe("formatConfigReference", () => {
  const output = formatConfigReference();

  test("includes header row", () => {
    expect(output).toContain("OPTION");
    expect(output).toContain("DESCRIPTION");
    expect(output).toContain("DEFAULT");
  });

  test("includes all top-level CONFIG_REFERENCE entries", () => {
    for (const opt of CONFIG_REFERENCE) {
      expect(output).toContain(opt.name);
    }
  });

  test("auto-expands codebases sub-fields", () => {
    expect(output).toContain("codebases[].relative_path");
    expect(output).toContain("codebases[].format");
    expect(output).toContain("codebases[].external");
  });

  test("auto-expands workspaces sub-fields", () => {
    expect(output).toContain("workspaces.<workspace>.gitBranch");
    expect(output).toContain("workspaces.<workspace>.baseUrl");
    expect(output).toContain("workspaces.<workspace>.workspaceId");
    expect(output).toContain("workspaces.<workspace>.specificItems.variables");
  });

  test("auto-expands commonSpecificItems sub-fields", () => {
    expect(output).toContain("workspaces.commonSpecificItems.variables");
    expect(output).toContain("workspaces.commonSpecificItems.settings");
  });

  test("deprecated entries are listed but not expanded", () => {
    expect(output).toContain("gitBranches");
    expect(output).toContain("[Deprecated]");
    // Should NOT have expanded sub-fields for deprecated entries
    expect(output).not.toContain("gitBranches.<workspace>");
  });
});

// =============================================================================
// formatConfigReferenceJson
// =============================================================================

describe("formatConfigReferenceJson", () => {
  test("produces valid JSON", () => {
    const parsed = JSON.parse(formatConfigReferenceJson());
    expect(Array.isArray(parsed)).toBe(true);
    expect(parsed.length).toBe(CONFIG_REFERENCE.length);
  });

  test("each entry has name, type, default, description", () => {
    const parsed = JSON.parse(formatConfigReferenceJson());
    for (const entry of parsed) {
      expect(entry).toHaveProperty("name");
      expect(entry).toHaveProperty("type");
      expect(entry).toHaveProperty("default");
      expect(entry).toHaveProperty("description");
    }
  });

  test("does not contain template-only keys", () => {
    const parsed = JSON.parse(formatConfigReferenceJson());
    const templateKeys = ["section", "sectionNote", "commented", "templateValue", "example", "inlineComment", "groupNote", "skipInTemplate"];
    for (const entry of parsed) {
      for (const key of templateKeys) {
        expect(entry).not.toHaveProperty(key);
      }
    }
  });
});

// =============================================================================
// CONFIG_REFERENCE integrity
// =============================================================================

describe("CONFIG_REFERENCE integrity", () => {
  test("all entries have required fields", () => {
    for (const opt of CONFIG_REFERENCE) {
      expect(opt.name).toBeTruthy();
      expect(opt.type).toBeTruthy();
      expect(opt.description).toBeTruthy();
      expect(opt.default).toBeDefined();
    }
  });

  test("no duplicate names", () => {
    const names = CONFIG_REFERENCE.map((o) => o.name);
    expect(new Set(names).size).toBe(names.length);
  });

  test("type field uses valid JSON Schema types", () => {
    const validTypes = new Set(["boolean", "string", "integer", "number", "array", "object"]);
    for (const opt of CONFIG_REFERENCE) {
      expect(validTypes.has(opt.type)).toBe(true);
    }
  });
});
