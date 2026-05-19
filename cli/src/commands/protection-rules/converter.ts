import { ProtectionRuleEntry } from "./types.ts";
import { ProtectionRuleset } from "../../../gen/types.gen.ts";

// Reconciliation plan produced by diffing the local protection rules
// against the backend list. `toDelete` holds names present on the backend but
// absent from wmill.yaml (full-reconcile semantics).
export interface ProtectionRulesPlan {
  toCreate: ProtectionRuleEntry[];
  toUpdate: ProtectionRuleEntry[];
  toDelete: string[];
  unchanged: string[];
}

function sortedUnique(arr: readonly string[]): string[] {
  return [...new Set(arr)].sort();
}

export class ProtectionRulesConverter {
  // Canonicalize a single rule so comparisons are insensitive to array order
  // and duplicates.
  static normalizeEntry(entry: ProtectionRuleEntry): ProtectionRuleEntry {
    return {
      name: entry.name,
      rules: sortedUnique(entry.rules ?? []) as ProtectionRuleEntry["rules"],
      bypass_groups: sortedUnique(entry.bypass_groups ?? []),
      bypass_users: sortedUnique(entry.bypass_users ?? []),
    };
  }

  // Canonicalize and sort a list of rules by name.
  static normalizeList(
    entries: ProtectionRuleEntry[] | undefined,
  ): ProtectionRuleEntry[] {
    return (entries ?? [])
      .map((e) => ProtectionRulesConverter.normalizeEntry(e))
      .sort((a, b) => a.name.localeCompare(b.name));
  }

  // Convert a backend ProtectionRuleset response into the wmill.yaml shape
  // (drops workspace_id, which is implied by the synced workspace).
  static fromBackend(rulesets: ProtectionRuleset[]): ProtectionRuleEntry[] {
    return ProtectionRulesConverter.normalizeList(
      rulesets.map((r) => ({
        name: r.name,
        rules: [...(r.rules ?? [])],
        bypass_groups: [...(r.bypass_groups ?? [])],
        bypass_users: [...(r.bypass_users ?? [])],
      })),
    );
  }

  static entriesEqual(
    a: ProtectionRuleEntry,
    b: ProtectionRuleEntry,
  ): boolean {
    const na = ProtectionRulesConverter.normalizeEntry(a);
    const nb = ProtectionRulesConverter.normalizeEntry(b);
    return (
      na.name === nb.name &&
      na.rules.length === nb.rules.length &&
      na.rules.every((v, i) => v === nb.rules[i]) &&
      na.bypass_groups.length === nb.bypass_groups.length &&
      na.bypass_groups.every((v, i) => v === nb.bypass_groups[i]) &&
      na.bypass_users.length === nb.bypass_users.length &&
      na.bypass_users.every((v, i) => v === nb.bypass_users[i])
    );
  }

  static listsEqual(
    a: ProtectionRuleEntry[] | undefined,
    b: ProtectionRuleEntry[] | undefined,
  ): boolean {
    const na = ProtectionRulesConverter.normalizeList(a);
    const nb = ProtectionRulesConverter.normalizeList(b);
    if (na.length !== nb.length) return false;
    return na.every((entry, i) =>
      ProtectionRulesConverter.entriesEqual(entry, nb[i])
    );
  }

  // Compute the create/update/delete plan to make `backend` match `local`.
  static computePlan(
    local: ProtectionRuleEntry[] | undefined,
    backend: ProtectionRuleEntry[] | undefined,
  ): ProtectionRulesPlan {
    const localByName = new Map(
      ProtectionRulesConverter.normalizeList(local).map((e) => [e.name, e]),
    );
    const backendByName = new Map(
      ProtectionRulesConverter.normalizeList(backend).map((e) => [e.name, e]),
    );

    const plan: ProtectionRulesPlan = {
      toCreate: [],
      toUpdate: [],
      toDelete: [],
      unchanged: [],
    };

    for (const [name, entry] of localByName) {
      const existing = backendByName.get(name);
      if (!existing) {
        plan.toCreate.push(entry);
      } else if (!ProtectionRulesConverter.entriesEqual(entry, existing)) {
        plan.toUpdate.push(entry);
      } else {
        plan.unchanged.push(name);
      }
    }

    for (const name of backendByName.keys()) {
      if (!localByName.has(name)) {
        plan.toDelete.push(name);
      }
    }

    plan.toCreate.sort((a, b) => a.name.localeCompare(b.name));
    plan.toUpdate.sort((a, b) => a.name.localeCompare(b.name));
    plan.toDelete.sort();
    plan.unchanged.sort();

    return plan;
  }

  static planHasChanges(plan: ProtectionRulesPlan): boolean {
    return (
      plan.toCreate.length > 0 ||
      plan.toUpdate.length > 0 ||
      plan.toDelete.length > 0
    );
  }
}
