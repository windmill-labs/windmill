/**
 * Unit tests for ProtectionRulesConverter.
 * Covers normalization, order-insensitive equality, and the
 * create/update/delete reconciliation plan used by pull/push.
 */

import { expect, test, describe } from "bun:test";
import { ProtectionRulesConverter } from "../src/commands/protection-rules/converter.ts";
import {
  applyRulesToBranchOverride,
  clearRuleOverride,
} from "../src/commands/protection-rules/utils.ts";
import { ProtectionRuleEntry, SyncOptions } from "../src/core/conf.ts";

const rule = (
  name: string,
  rules: ProtectionRuleEntry["rules"],
  groups: string[] = [],
  users: string[] = [],
): ProtectionRuleEntry => ({
  name,
  rules,
  bypass_groups: groups,
  bypass_users: users,
});

describe("normalizeEntry", () => {
  test("sorts and dedupes rules, groups, users", () => {
    const r = rule(
      "prod",
      ["RestrictDeployToDeployers", "DisableDirectDeployment", "DisableDirectDeployment"],
      ["g/b", "g/a"],
      ["u/y", "u/x", "u/x"],
    );
    const n = ProtectionRulesConverter.normalizeEntry(r);
    expect(n.rules).toEqual([
      "DisableDirectDeployment",
      "RestrictDeployToDeployers",
    ]);
    expect(n.bypass_groups).toEqual(["g/a", "g/b"]);
    expect(n.bypass_users).toEqual(["u/x", "u/y"]);
  });

  test("handles missing arrays", () => {
    const n = ProtectionRulesConverter.normalizeEntry({
      name: "x",
    } as unknown as ProtectionRuleEntry);
    expect(n.rules).toEqual([]);
    expect(n.bypass_groups).toEqual([]);
    expect(n.bypass_users).toEqual([]);
  });
});

describe("entriesEqual", () => {
  test("equal regardless of array order", () => {
    const a = rule("p", ["DisableDirectDeployment", "DisableWorkspaceForking"], ["g/a", "g/b"]);
    const b = rule("p", ["DisableWorkspaceForking", "DisableDirectDeployment"], ["g/b", "g/a"]);
    expect(ProtectionRulesConverter.entriesEqual(a, b)).toBe(true);
  });

  test("different rules are not equal", () => {
    const a = rule("p", ["DisableDirectDeployment"]);
    const b = rule("p", ["DisableWorkspaceForking"]);
    expect(ProtectionRulesConverter.entriesEqual(a, b)).toBe(false);
  });

  test("different bypass users are not equal", () => {
    const a = rule("p", ["DisableDirectDeployment"], [], ["u/a"]);
    const b = rule("p", ["DisableDirectDeployment"], [], ["u/b"]);
    expect(ProtectionRulesConverter.entriesEqual(a, b)).toBe(false);
  });
});

describe("fromBackend", () => {
  test("strips workspace_id and normalizes", () => {
    const out = ProtectionRulesConverter.fromBackend([
      {
        name: "p",
        workspace_id: "ws1",
        rules: ["DisableWorkspaceForking", "DisableDirectDeployment"],
        bypass_groups: ["g/b", "g/a"],
        bypass_users: [],
      },
    ]);
    expect(out).toEqual([
      {
        name: "p",
        rules: ["DisableDirectDeployment", "DisableWorkspaceForking"],
        bypass_groups: ["g/a", "g/b"],
        bypass_users: [],
      },
    ]);
  });
});

describe("listsEqual", () => {
  test("equal regardless of list order", () => {
    const a = [rule("a", ["DisableDirectDeployment"]), rule("b", ["DisableWorkspaceForking"])];
    const b = [rule("b", ["DisableWorkspaceForking"]), rule("a", ["DisableDirectDeployment"])];
    expect(ProtectionRulesConverter.listsEqual(a, b)).toBe(true);
  });

  test("undefined equals empty", () => {
    expect(ProtectionRulesConverter.listsEqual(undefined, [])).toBe(true);
  });

  test("different length not equal", () => {
    expect(
      ProtectionRulesConverter.listsEqual([rule("a", [])], []),
    ).toBe(false);
  });
});

describe("computePlan (full reconcile)", () => {
  test("creates rules present locally but not on backend", () => {
    const plan = ProtectionRulesConverter.computePlan(
      [rule("new", ["DisableDirectDeployment"])],
      [],
    );
    expect(plan.toCreate.map((e) => e.name)).toEqual(["new"]);
    expect(plan.toUpdate).toEqual([]);
    expect(plan.toDelete).toEqual([]);
  });

  test("deletes backend rules not present locally", () => {
    const plan = ProtectionRulesConverter.computePlan(
      [],
      [rule("stale", ["DisableDirectDeployment"])],
    );
    expect(plan.toDelete).toEqual(["stale"]);
    expect(plan.toCreate).toEqual([]);
  });

  test("updates rules whose content changed", () => {
    const plan = ProtectionRulesConverter.computePlan(
      [rule("p", ["DisableDirectDeployment", "DisableWorkspaceForking"])],
      [rule("p", ["DisableDirectDeployment"])],
    );
    expect(plan.toUpdate.map((e) => e.name)).toEqual(["p"]);
    expect(plan.toCreate).toEqual([]);
    expect(plan.toDelete).toEqual([]);
  });

  test("unchanged rules are not in create/update/delete", () => {
    const same = [rule("p", ["DisableDirectDeployment"], ["g/a"])];
    const plan = ProtectionRulesConverter.computePlan(same, [
      rule("p", ["DisableDirectDeployment"], ["g/a"]),
    ]);
    expect(ProtectionRulesConverter.planHasChanges(plan)).toBe(false);
    expect(plan.unchanged).toEqual(["p"]);
  });

  test("mixed plan: create + update + delete + unchanged", () => {
    const local = [
      rule("keep", ["DisableDirectDeployment"]),
      rule("change", ["DisableWorkspaceForking"]),
      rule("brand-new", ["RestrictDeployToDeployers"]),
    ];
    const backend = [
      rule("keep", ["DisableDirectDeployment"]),
      rule("change", ["DisableDirectDeployment"]),
      rule("gone", ["DisableDirectDeployment"]),
    ];
    const plan = ProtectionRulesConverter.computePlan(local, backend);
    expect(plan.toCreate.map((e) => e.name)).toEqual(["brand-new"]);
    expect(plan.toUpdate.map((e) => e.name)).toEqual(["change"]);
    expect(plan.toDelete).toEqual(["gone"]);
    expect(plan.unchanged).toEqual(["keep"]);
    expect(ProtectionRulesConverter.planHasChanges(plan)).toBe(true);
  });

  test("reordered arrays do not produce spurious updates", () => {
    const plan = ProtectionRulesConverter.computePlan(
      [rule("p", ["DisableWorkspaceForking", "DisableDirectDeployment"], ["g/b", "g/a"])],
      [rule("p", ["DisableDirectDeployment", "DisableWorkspaceForking"], ["g/a", "g/b"])],
    );
    expect(ProtectionRulesConverter.planHasChanges(plan)).toBe(false);
  });
});

describe("applyRulesToBranchOverride", () => {
  test("writes under the given resolved workspace key, not a raw branch", () => {
    // workspaces.prod maps to git branch main; the override MUST land on
    // `prod` (the key getEffectiveSettings resolves), not `main`.
    const config: SyncOptions = {
      workspaces: { prod: { gitBranch: "main" } } as any,
    };
    const out = applyRulesToBranchOverride(config, "prod", [
      rule("r", ["DisableDirectDeployment"]),
    ]);
    expect((out.workspaces as any).prod.overrides.protectionRules).toEqual([
      rule("r", ["DisableDirectDeployment"]),
    ]);
    expect((out.workspaces as any).main).toBeUndefined();
  });

  test("creates the workspace entry if missing", () => {
    const out = applyRulesToBranchOverride({}, "feature", [rule("a", [])]);
    expect((out.workspaces as any).feature.overrides.protectionRules).toEqual([
      rule("a", []),
    ]);
  });
});

describe("clearRuleOverride", () => {
  test("removes protectionRules from overrides and promotionOverrides", () => {
    const config: SyncOptions = {
      workspaces: {
        prod: {
          gitBranch: "main",
          overrides: { protectionRules: [rule("a", [])], skipScripts: true },
          promotionOverrides: { protectionRules: [rule("b", [])] },
        },
      } as any,
    };
    expect(clearRuleOverride(config, "prod")).toBe(true);
    expect(
      "protectionRules" in (config.workspaces as any).prod.overrides,
    ).toBe(false);
    // unrelated override keys are preserved
    expect((config.workspaces as any).prod.overrides.skipScripts).toBe(true);
    expect(
      "protectionRules" in (config.workspaces as any).prod.promotionOverrides,
    ).toBe(false);
  });

  test("returns false when there is nothing to clear", () => {
    expect(clearRuleOverride({ workspaces: { prod: {} } as any }, "prod")).toBe(
      false,
    );
    expect(clearRuleOverride({}, "missing")).toBe(false);
  });
});
