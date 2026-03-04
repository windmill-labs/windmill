/**
 * Unit tests for permissioned_as.ts: rule validation, rule resolution, and pre-check logic.
 */

import { expect, test, describe } from "bun:test";
import {
  validatePermissionedAsRules,
  resolvePermissionedAsEmail,
  type PermissionedAsRule,
} from "../src/core/permissioned_as.ts";

// =============================================================================
// validatePermissionedAsRules
// =============================================================================

describe("validatePermissionedAsRules", () => {
  test("returns empty array for undefined input", () => {
    expect(validatePermissionedAsRules(undefined, "wmill.yaml")).toEqual([]);
  });

  test("returns empty array for null input", () => {
    expect(validatePermissionedAsRules(null, "wmill.yaml")).toEqual([]);
  });

  test("returns empty array for empty array input", () => {
    expect(validatePermissionedAsRules([], "wmill.yaml")).toEqual([]);
  });

  test("validates a correct rule", () => {
    const rules = [{ email: "admin@company.com", path_pattern: "f/**" }];
    const result = validatePermissionedAsRules(rules, "wmill.yaml");
    expect(result).toEqual([
      { email: "admin@company.com", path_pattern: "f/**" },
    ]);
  });

  test("validates multiple correct rules", () => {
    const rules = [
      { email: "admin@company.com", path_pattern: "f/production/**" },
      { email: "deploy@company.com", path_pattern: "f/**" },
    ];
    const result = validatePermissionedAsRules(rules, "wmill.yaml");
    expect(result).toHaveLength(2);
    expect(result[0].email).toBe("admin@company.com");
    expect(result[1].email).toBe("deploy@company.com");
  });

  test("throws on non-array input", () => {
    expect(() =>
      validatePermissionedAsRules("not-an-array", "wmill.yaml")
    ).toThrow("expected an array of rules, got string");
  });

  test("throws on object input", () => {
    expect(() =>
      validatePermissionedAsRules(
        { email: "a@b.com", path_pattern: "f/**" },
        "wmill.yaml"
      )
    ).toThrow("expected an array of rules, got object");
  });

  test("throws on non-object rule entry", () => {
    expect(() =>
      validatePermissionedAsRules(["not-an-object"], "wmill.yaml")
    ).toThrow(
      "defaultPermissionedAs[0] in wmill.yaml: expected an object with 'email' and 'path_pattern' fields"
    );
  });

  test("throws on null rule entry", () => {
    expect(() =>
      validatePermissionedAsRules([null], "wmill.yaml")
    ).toThrow(
      "defaultPermissionedAs[0] in wmill.yaml: expected an object with 'email' and 'path_pattern' fields"
    );
  });

  // --- Unknown/misspelled fields ---

  test("throws on misspelled 'email' field", () => {
    expect(() =>
      validatePermissionedAsRules(
        [{ emaill: "a@b.com", path_pattern: "f/**" }],
        "wmill.yaml"
      )
    ).toThrow("unknown field(s) 'emaill'");
  });

  test("throws on misspelled 'path_pattern' field", () => {
    expect(() =>
      validatePermissionedAsRules(
        [{ email: "a@b.com", pattern: "f/**" }],
        "wmill.yaml"
      )
    ).toThrow("unknown field(s) 'pattern'");
  });

  test("throws on extra unknown field", () => {
    expect(() =>
      validatePermissionedAsRules(
        [{ email: "a@b.com", path_pattern: "f/**", extra_field: true }],
        "wmill.yaml"
      )
    ).toThrow("unknown field(s) 'extra_field'");
  });

  test("throws listing multiple unknown fields", () => {
    expect(() =>
      validatePermissionedAsRules(
        [{ emaill: "a@b.com", pattern: "f/**" }],
        "wmill.yaml"
      )
    ).toThrow("unknown field(s) 'emaill', 'pattern'");
  });

  test("error message includes valid field names", () => {
    expect(() =>
      validatePermissionedAsRules(
        [{ emaill: "a@b.com", path_pattern: "f/**" }],
        "wmill.yaml"
      )
    ).toThrow("Valid fields are: 'email', 'path_pattern'");
  });

  // --- Missing required fields ---

  test("throws on missing email", () => {
    expect(() =>
      validatePermissionedAsRules(
        [{ path_pattern: "f/**" }],
        "wmill.yaml"
      )
    ).toThrow("'email' is required and must be a non-empty string");
  });

  test("throws on empty email", () => {
    expect(() =>
      validatePermissionedAsRules(
        [{ email: "", path_pattern: "f/**" }],
        "wmill.yaml"
      )
    ).toThrow("'email' is required and must be a non-empty string");
  });

  test("throws on non-string email", () => {
    expect(() =>
      validatePermissionedAsRules(
        [{ email: 123, path_pattern: "f/**" }],
        "wmill.yaml"
      )
    ).toThrow("'email' is required and must be a non-empty string");
  });

  test("throws on missing path_pattern", () => {
    expect(() =>
      validatePermissionedAsRules(
        [{ email: "a@b.com" }],
        "wmill.yaml"
      )
    ).toThrow("'path_pattern' is required and must be a non-empty string");
  });

  test("throws on empty path_pattern", () => {
    expect(() =>
      validatePermissionedAsRules(
        [{ email: "a@b.com", path_pattern: "" }],
        "wmill.yaml"
      )
    ).toThrow("'path_pattern' is required and must be a non-empty string");
  });

  test("throws on non-string path_pattern", () => {
    expect(() =>
      validatePermissionedAsRules(
        [{ email: "a@b.com", path_pattern: 42 }],
        "wmill.yaml"
      )
    ).toThrow("'path_pattern' is required and must be a non-empty string");
  });

  // --- Error index ---

  test("error message includes rule index", () => {
    expect(() =>
      validatePermissionedAsRules(
        [
          { email: "a@b.com", path_pattern: "f/**" },
          { email: "", path_pattern: "f/**" },
        ],
        "wmill.yaml"
      )
    ).toThrow("defaultPermissionedAs[1]");
  });

  // --- Source label ---

  test("error message includes source label", () => {
    expect(() =>
      validatePermissionedAsRules("bad", "gitBranches.main")
    ).toThrow("in gitBranches.main");
  });
});

// =============================================================================
// resolvePermissionedAsEmail
// =============================================================================

describe("resolvePermissionedAsEmail", () => {
  const rules: PermissionedAsRule[] = [
    { email: "prod@company.com", path_pattern: "f/production/**" },
    { email: "staging@company.com", path_pattern: "f/staging/**" },
    { email: "default@company.com", path_pattern: "f/**" },
  ];

  test("returns matching email for specific path", () => {
    expect(
      resolvePermissionedAsEmail("f/production/my_script", rules)
    ).toBe("prod@company.com");
  });

  test("returns first matching rule (production over default)", () => {
    expect(
      resolvePermissionedAsEmail("f/production/deep/nested", rules)
    ).toBe("prod@company.com");
  });

  test("returns staging email for staging path", () => {
    expect(
      resolvePermissionedAsEmail("f/staging/my_flow", rules)
    ).toBe("staging@company.com");
  });

  test("falls through to default rule", () => {
    expect(
      resolvePermissionedAsEmail("f/other/my_script", rules)
    ).toBe("default@company.com");
  });

  test("returns undefined when no rule matches", () => {
    expect(
      resolvePermissionedAsEmail("u/admin/my_script", rules)
    ).toBeUndefined();
  });

  test("returns undefined for empty rules", () => {
    expect(resolvePermissionedAsEmail("f/anything", [])).toBeUndefined();
  });

  test("handles exact path patterns", () => {
    const exactRules: PermissionedAsRule[] = [
      { email: "exact@company.com", path_pattern: "f/specific/script" },
    ];
    expect(
      resolvePermissionedAsEmail("f/specific/script", exactRules)
    ).toBe("exact@company.com");
    expect(
      resolvePermissionedAsEmail("f/specific/other", exactRules)
    ).toBeUndefined();
  });

  test("handles single-level wildcard", () => {
    const wildcardRules: PermissionedAsRule[] = [
      { email: "wild@company.com", path_pattern: "f/*/scripts" },
    ];
    expect(
      resolvePermissionedAsEmail("f/team_a/scripts", wildcardRules)
    ).toBe("wild@company.com");
    expect(
      resolvePermissionedAsEmail("f/team_a/nested/scripts", wildcardRules)
    ).toBeUndefined();
  });
});
