import { describe, expect, test } from "bun:test";
import { branchToForkId } from "../src/commands/workspace/fork.ts";

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
});
