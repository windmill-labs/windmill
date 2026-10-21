import { describe, expect, test } from "bun:test";
import { folderRuleGlobMatches } from "../src/utils/folder_rule_glob.ts";

// Expected matches are what `globset::Glob::new(glob).compile_matcher()` answers on the server.
const PATHS = ["x", "jobs/a", "jobs/a/b", "a/b", "a/x/b", "a/c", "abc", "foo/z", "bar/z/q", "baz/z"];
const CASES: [string, string[]][] = [
  ["**", PATHS],
  ["*", PATHS],
  ["**/", PATHS],
  ["jobs/*", ["jobs/a", "jobs/a/b"]],
  ["a/**/b", ["a/b", "a/x/b"]],
  ["**/x", ["x"]],
  ["a?c", ["a/c", "abc"]],
  ["[!ab]*", ["x", "jobs/a", "jobs/a/b", "foo/z"]],
  ["{foo,bar}/*", ["foo/z", "bar/z/q"]],
  ["[unclosed", []],
];

describe("folderRuleGlobMatches", () => {
  for (const [glob, expected] of CASES) {
    test(`${glob} matches what the server matches`, () => {
      expect(PATHS.filter((p) => folderRuleGlobMatches(glob, p))).toEqual(expected);
    });
  }
});
