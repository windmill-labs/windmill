import { describe, expect, test } from "bun:test";
import {
  branchToForkId,
  validateForkWorkspaceId,
} from "../src/commands/workspace/fork.ts";

describe("branchToForkId — branch name → fork workspace id slug", () => {
  test.each<[string, string]>([
    ["feature-x", "feature-x"],
    ["my_feature", "my_feature"],
    // `/` (common in branch names) must not survive — it would break the
    // wm-fork/<base>/<id> branch-name parsing.
    ["feat/foo", "feat-foo"],
    ["feature/JIRA-123", "feature-JIRA-123"],
    ["a/b/c", "a-b-c"],
    // Other invalid characters collapse to a single dash.
    ["hot fix!", "hot-fix"],
    ["weird@@name", "weird-name"],
    // Leading/trailing separators are trimmed.
    ["/leading", "leading"],
    ["trailing/", "trailing"],
    ["--dashes--", "dashes"],
    // Degenerate input falls back to a usable id.
    ["///", "fork"],
    ["", "fork"],
  ])("%p → %p", (branch, expected) => {
    expect(branchToForkId(branch)).toBe(expected);
  });

  test("result never contains a slash (would break fork branch parsing)", () => {
    for (const branch of ["a/b", "x/y/z", "feat/foo/bar"]) {
      expect(branchToForkId(branch)).not.toContain("/");
    }
  });

  test("caps the slug so wm-fork-<slug> stays within the backend's 50-char limit", () => {
    const long = "feature/TICKET-1234-" + "a".repeat(80);
    const slug = branchToForkId(long);
    expect(slug.length).toBeLessThanOrEqual(42);
    // The full id the backend validates is `wm-fork-<slug>`.
    expect(`wm-fork-${slug}`.length).toBeLessThanOrEqual(50);
    // Truncation must not leave a trailing dash.
    expect(slug.endsWith("-")).toBe(false);
  });
});

describe("validateForkWorkspaceId — mirrors backend validate_fork_workspace_id", () => {
  test("accepts a normal slugged id", () => {
    expect(() => validateForkWorkspaceId("wm-fork-feature-x")).not.toThrow();
  });

  test.each<[string, string]>([
    ["too long (> 50 chars)", "wm-fork-" + "a".repeat(60)],
    ["ends with '.'", "wm-fork-foo."],
    ["ends with '.lock'", "wm-fork-foo.lock"],
    ["contains '..'", "wm-fork-foo..bar"],
    ["contains '//'", "wm-fork-foo//bar"],
    ["contains a space", "wm-fork-foo bar"],
    ["contains a forbidden char", "wm-fork-foo~bar"],
  ])("rejects: %s", (_label, id) => {
    expect(() => validateForkWorkspaceId(id)).toThrow();
  });

  test("a branchToForkId slug always passes validation (with the prefix)", () => {
    for (const branch of [
      "feat/foo",
      "feature/TICKET-1234-" + "a".repeat(80),
      "weird@@name",
      "///",
    ]) {
      const id = `wm-fork-${branchToForkId(branch)}`;
      expect(() => validateForkWorkspaceId(id)).not.toThrow();
    }
  });
});
