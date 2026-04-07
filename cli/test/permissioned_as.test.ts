/**
 * Unit tests for permissioned_as.ts: rule validation, rule resolution, and pre-check logic.
 */

import { expect, test, describe } from "bun:test";
import { mock } from "bun:test";
import {
  validatePermissionedAsRules,
  resolvePermissionedAsRule,
  preCheckPermissionedAs,
  type PermissionedAsRule,
  type Change,
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
    const rules = [{ username: "u/admin", path_pattern: "f/**" }];
    const result = validatePermissionedAsRules(rules, "wmill.yaml");
    expect(result).toEqual([
      { username: "u/admin", path_pattern: "f/**" },
    ]);
  });

  test("validates multiple correct rules", () => {
    const rules = [
      { username: "u/admin", path_pattern: "f/production/**" },
      { username: "u/deploy", path_pattern: "f/**" },
    ];
    const result = validatePermissionedAsRules(rules, "wmill.yaml");
    expect(result).toHaveLength(2);
    expect(result[0].username).toBe("u/admin");
    expect(result[1].username).toBe("u/deploy");
  });

  test("throws on non-array input", () => {
    expect(() =>
      validatePermissionedAsRules("not-an-array", "wmill.yaml")
    ).toThrow("expected an array of rules, got string");
  });

  test("throws on object input", () => {
    expect(() =>
      validatePermissionedAsRules(
        { username: "u/admin", path_pattern: "f/**" },
        "wmill.yaml"
      )
    ).toThrow("expected an array of rules, got object");
  });

  test("throws on non-object rule entry", () => {
    expect(() =>
      validatePermissionedAsRules(["not-an-object"], "wmill.yaml")
    ).toThrow(
      "defaultPermissionedAs[0] in wmill.yaml: expected an object with 'username' and 'path_pattern' fields"
    );
  });

  test("throws on null rule entry", () => {
    expect(() =>
      validatePermissionedAsRules([null], "wmill.yaml")
    ).toThrow(
      "defaultPermissionedAs[0] in wmill.yaml: expected an object with 'username' and 'path_pattern' fields"
    );
  });

  // --- Unknown/misspelled fields ---

  test("throws on misspelled 'username' field", () => {
    expect(() =>
      validatePermissionedAsRules(
        [{ usernamee: "u/admin", path_pattern: "f/**" }],
        "wmill.yaml"
      )
    ).toThrow("unknown field(s) 'usernamee'");
  });

  test("throws on misspelled 'path_pattern' field", () => {
    expect(() =>
      validatePermissionedAsRules(
        [{ username: "u/admin", pattern: "f/**" }],
        "wmill.yaml"
      )
    ).toThrow("unknown field(s) 'pattern'");
  });

  test("throws on extra unknown field", () => {
    expect(() =>
      validatePermissionedAsRules(
        [{ username: "u/admin", path_pattern: "f/**", extra_field: true }],
        "wmill.yaml"
      )
    ).toThrow("unknown field(s) 'extra_field'");
  });

  test("throws listing multiple unknown fields", () => {
    expect(() =>
      validatePermissionedAsRules(
        [{ usernamee: "u/admin", pattern: "f/**" }],
        "wmill.yaml"
      )
    ).toThrow("unknown field(s) 'usernamee', 'pattern'");
  });

  test("error message includes valid field names", () => {
    expect(() =>
      validatePermissionedAsRules(
        [{ usernamee: "u/admin", path_pattern: "f/**" }],
        "wmill.yaml"
      )
    ).toThrow("Valid fields are: 'username', 'path_pattern'");
  });

  // --- Missing required fields ---

  test("throws on missing username", () => {
    expect(() =>
      validatePermissionedAsRules(
        [{ path_pattern: "f/**" }],
        "wmill.yaml"
      )
    ).toThrow("'username' is required and must be a non-empty string");
  });

  test("throws on empty username", () => {
    expect(() =>
      validatePermissionedAsRules(
        [{ username: "", path_pattern: "f/**" }],
        "wmill.yaml"
      )
    ).toThrow("'username' is required and must be a non-empty string");
  });

  test("throws on non-string username", () => {
    expect(() =>
      validatePermissionedAsRules(
        [{ username: 123, path_pattern: "f/**" }],
        "wmill.yaml"
      )
    ).toThrow("'username' is required and must be a non-empty string");
  });

  test("throws on missing path_pattern", () => {
    expect(() =>
      validatePermissionedAsRules(
        [{ username: "u/admin" }],
        "wmill.yaml"
      )
    ).toThrow("'path_pattern' is required and must be a non-empty string");
  });

  test("throws on empty path_pattern", () => {
    expect(() =>
      validatePermissionedAsRules(
        [{ username: "u/admin", path_pattern: "" }],
        "wmill.yaml"
      )
    ).toThrow("'path_pattern' is required and must be a non-empty string");
  });

  test("throws on non-string path_pattern", () => {
    expect(() =>
      validatePermissionedAsRules(
        [{ username: "u/admin", path_pattern: 42 }],
        "wmill.yaml"
      )
    ).toThrow("'path_pattern' is required and must be a non-empty string");
  });

  // --- Error index ---

  test("error message includes rule index", () => {
    expect(() =>
      validatePermissionedAsRules(
        [
          { username: "u/admin", path_pattern: "f/**" },
          { username: "", path_pattern: "f/**" },
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
// resolvePermissionedAsRule
// =============================================================================

describe("resolvePermissionedAsRule", () => {
  const rules: PermissionedAsRule[] = [
    { username: "u/prod", path_pattern: "f/production/**" },
    { username: "u/staging", path_pattern: "f/staging/**" },
    { username: "u/default", path_pattern: "f/**" },
  ];

  test("returns matching rule for specific path", () => {
    const rule = resolvePermissionedAsRule("f/production/my_script", rules);
    expect(rule?.username).toBe("u/prod");
    expect(rule?.path_pattern).toBe("f/production/**");
  });

  test("returns first matching rule (production over default)", () => {
    const rule = resolvePermissionedAsRule("f/production/deep/nested", rules);
    expect(rule?.username).toBe("u/prod");
    expect(rule?.path_pattern).toBe("f/production/**");
  });

  test("returns staging rule for staging path", () => {
    const rule = resolvePermissionedAsRule("f/staging/my_flow", rules);
    expect(rule?.username).toBe("u/staging");
    expect(rule?.path_pattern).toBe("f/staging/**");
  });

  test("falls through to default rule", () => {
    const rule = resolvePermissionedAsRule("f/other/my_script", rules);
    expect(rule?.username).toBe("u/default");
    expect(rule?.path_pattern).toBe("f/**");
  });

  test("returns undefined when no rule matches", () => {
    expect(
      resolvePermissionedAsRule("u/admin/my_script", rules)
    ).toBeUndefined();
  });

  test("returns undefined for empty rules", () => {
    expect(resolvePermissionedAsRule("f/anything", [])).toBeUndefined();
  });

  test("handles exact path patterns", () => {
    const exactRules: PermissionedAsRule[] = [
      { username: "u/exact", path_pattern: "f/specific/script" },
    ];
    const rule = resolvePermissionedAsRule("f/specific/script", exactRules);
    expect(rule?.username).toBe("u/exact");
    expect(
      resolvePermissionedAsRule("f/specific/other", exactRules)
    ).toBeUndefined();
  });

  test("handles single-level wildcard", () => {
    const wildcardRules: PermissionedAsRule[] = [
      { username: "u/wild", path_pattern: "f/*/scripts" },
    ];
    const rule = resolvePermissionedAsRule("f/team_a/scripts", wildcardRules);
    expect(rule?.username).toBe("u/wild");
    expect(
      resolvePermissionedAsRule("f/team_a/nested/scripts", wildcardRules)
    ).toBeUndefined();
  });
});

// =============================================================================
// preCheckPermissionedAs — has_on_behalf_of gating
// =============================================================================

describe("preCheckPermissionedAs", () => {
  const userEmail = "user@example.com";

  // Helper to check if preCheck would exit (flag items)
  async function expectFlagged(fn: () => Promise<void>) {
    const originalExit = process.exit;
    let exitCalled = false;
    process.exit = ((code?: number) => { exitCalled = true; }) as any;
    try {
      await fn();
      expect(exitCalled).toBe(true);
    } finally {
      process.exit = originalExit;
    }
  }

  // Helper: make a script edit change
  function scriptEdit(before: string): Change {
    return {
      name: "edited",
      path: "f/my_script.script.yaml",
      before,
      after: "summary: updated\n",
    };
  }

  // Helper: make a script added change
  function scriptAdded(content: string, path = "f/my_script.script.yaml"): Change {
    return { name: "added", path, content };
  }

  // Helper: make a flow edit change
  function flowEdit(before: string): Change {
    return {
      name: "edited",
      path: "f/my_flow.flow/flow.yaml",
      before,
      after: "summary: updated\n",
    };
  }

  // Helper: make a flow added change
  function flowAdded(content: string, path = "f/my_flow.flow/flow.yaml"): Change {
    return { name: "added", path, content };
  }

  // --- Non-admin, edited changes ---

  test("non-admin: edited script with has_on_behalf_of: true is flagged", async () => {
    await expectFlagged(() =>
      preCheckPermissionedAs([scriptEdit("summary: test\nhas_on_behalf_of: true\n")], userEmail, false, false, false)
    );
  });

  test("non-admin: edited script with has_on_behalf_of: false is not flagged", async () => {
    await preCheckPermissionedAs([scriptEdit("summary: test\nhas_on_behalf_of: false\n")], userEmail, false, false, false);
  });

  test("non-admin: edited flow with has_on_behalf_of: true is flagged", async () => {
    await expectFlagged(() =>
      preCheckPermissionedAs([flowEdit("summary: test\nhas_on_behalf_of: true\n")], userEmail, false, false, false)
    );
  });

  test("non-admin: edited flow with has_on_behalf_of: false is not flagged", async () => {
    await preCheckPermissionedAs([flowEdit("summary: test\nhas_on_behalf_of: false\n")], userEmail, false, false, false);
  });

  test("non-admin: legacy script with on_behalf_of_email is still flagged", async () => {
    await expectFlagged(() =>
      preCheckPermissionedAs([scriptEdit("summary: test\non_behalf_of_email: foo@bar.com\n")], userEmail, false, false, false)
    );
  });

  test("non-admin: script without obo fields is not flagged", async () => {
    await preCheckPermissionedAs([scriptEdit("summary: test\n")], userEmail, false, false, false);
  });

  // --- Non-admin, added changes ---

  test("non-admin: added script with has_on_behalf_of: true is flagged", async () => {
    await expectFlagged(() =>
      preCheckPermissionedAs([scriptAdded("summary: test\nhas_on_behalf_of: true\n")], userEmail, false, false, false)
    );
  });

  test("non-admin: added flow with has_on_behalf_of: true is flagged", async () => {
    await expectFlagged(() =>
      preCheckPermissionedAs([flowAdded("summary: test\nhas_on_behalf_of: true\n")], userEmail, false, false, false)
    );
  });

  test("non-admin: added script with has_on_behalf_of: false is not flagged", async () => {
    await preCheckPermissionedAs([scriptAdded("summary: test\nhas_on_behalf_of: false\n")], userEmail, false, false, false);
  });

  // --- Admin, edited changes (preserve handles these — not flagged) ---

  test("admin: edited script with has_on_behalf_of: true is not flagged (preserve handles)", async () => {
    await preCheckPermissionedAs([scriptEdit("summary: test\nhas_on_behalf_of: true\n")], userEmail, true, false, false);
  });

  test("admin: edited flow with has_on_behalf_of: true is not flagged (preserve handles)", async () => {
    await preCheckPermissionedAs([flowEdit("summary: test\nhas_on_behalf_of: true\n")], userEmail, true, false, false);
  });

  // --- Admin, added changes (no remote to preserve — rule check) ---

  test("admin: added script with has_on_behalf_of: true and no rule is flagged", async () => {
    await expectFlagged(() =>
      preCheckPermissionedAs([scriptAdded("summary: test\nhas_on_behalf_of: true\n")], userEmail, true, false, false, [])
    );
  });

  test("admin: added flow with has_on_behalf_of: true and no rule is flagged", async () => {
    await expectFlagged(() =>
      preCheckPermissionedAs([flowAdded("summary: test\nhas_on_behalf_of: true\n")], userEmail, true, false, false, [])
    );
  });

  test("admin: added script with has_on_behalf_of: true and matching rule is not flagged", async () => {
    const rules = [{ username: "u/admin", path_pattern: "f/**" }];
    await preCheckPermissionedAs([scriptAdded("summary: test\nhas_on_behalf_of: true\n")], userEmail, true, false, false, rules);
  });

  test("admin: added flow with has_on_behalf_of: true and matching rule is not flagged", async () => {
    const rules = [{ username: "u/admin", path_pattern: "f/**" }];
    await preCheckPermissionedAs([flowAdded("summary: test\nhas_on_behalf_of: true\n")], userEmail, true, false, false, rules);
  });

  test("admin: added script with has_on_behalf_of: false is not flagged (no obo)", async () => {
    await preCheckPermissionedAs([scriptAdded("summary: test\nhas_on_behalf_of: false\n")], userEmail, true, false, false, []);
  });

  // --- acceptOverride flag ---

  test("flagged items with acceptOverride: true logs warning but does not exit", async () => {
    // Should return normally (warning logged but no exit)
    await preCheckPermissionedAs(
      [scriptAdded("summary: test\nhas_on_behalf_of: true\n")],
      userEmail, true, true, false, []
    );
  });
});
