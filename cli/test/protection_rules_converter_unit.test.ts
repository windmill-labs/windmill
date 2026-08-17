/**
 * Unit tests for the protection-rules feature: the reconciliation converter,
 * the WorkspaceResolver (protection-rules.yaml key -> backend id via
 * wmill.yaml), and protection-rules.yaml read/write round-tripping.
 */

import { expect, test, describe } from "bun:test";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { mkdtempSync, rmSync } from "node:fs";
import { ProtectionRulesConverter } from "../src/commands/protection-rules/converter.ts";
import {
  WorkspaceResolver,
  readProtectionRulesFile,
  writeProtectionRulesFile,
} from "../src/commands/protection-rules/file.ts";
import { ProtectionRuleEntry } from "../src/commands/protection-rules/types.ts";
import { SyncOptions } from "../src/core/conf.ts";

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

describe("WorkspaceResolver", () => {
  const config: SyncOptions = {
    workspaces: {
      prod: { workspaceId: "acme-prod" },
      dev: {},
      commonSpecificItems: { settings: true },
    } as any,
  };
  const r = WorkspaceResolver.fromConfig(config);

  test("knownNames excludes reserved keys", () => {
    expect(r.knownNames().sort()).toEqual(["dev", "prod"]);
  });

  test("backendId uses workspaceId when set, else the key name", () => {
    expect(r.backendId("prod")).toBe("acme-prod");
    expect(r.backendId("dev")).toBe("dev");
  });

  test("backendId throws for a key absent from wmill.yaml", () => {
    expect(() => r.backendId("ghost")).toThrow(/not defined in wmill\.yaml/);
  });

  test("has reflects membership", () => {
    expect(r.has("prod")).toBe(true);
    expect(r.has("ghost")).toBe(false);
  });

  test("empty config resolves to no workspaces", () => {
    expect(WorkspaceResolver.fromConfig({}).knownNames()).toEqual([]);
  });
});

describe("protection-rules.yaml read/write", () => {
  test("round-trips and sorts workspace keys deterministically", async () => {
    const dir = mkdtempSync(join(tmpdir(), "prfile-"));
    const path = join(dir, "protection-rules.yaml");
    try {
      expect(await readProtectionRulesFile(path)).toEqual({});

      await writeProtectionRulesFile(path, {
        prod: [rule("p", ["DisableDirectDeployment"], ["g/a"])],
        dev: [],
      });
      const back = await readProtectionRulesFile(path);
      expect(Object.keys(back)).toEqual(["dev", "prod"]);
      expect(back.prod).toEqual([
        rule("p", ["DisableDirectDeployment"], ["g/a"]),
      ]);
      expect(back.dev).toEqual([]);
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });
});
