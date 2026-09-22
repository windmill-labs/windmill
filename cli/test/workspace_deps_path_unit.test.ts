import { expect, test } from "bun:test";
import { isWorkspaceDependencies } from "../src/utils/utils.ts";
import { workspaceDependenciesPathToLanguageAndFilename } from "../src/utils/script_common.ts";

// Sync paths carry the platform separator. A Windows path parsed to a different
// set name pushes the file to a set the workspace does not have.
test("workspace dependency paths parse the same with either separator", () => {
  for (const sep of ["/", "\\"]) {
    expect(isWorkspaceDependencies(`dependencies${sep}requirements.in`)).toBe(true);
    expect(
      workspaceDependenciesPathToLanguageAndFilename(`dependencies${sep}requirements.in`),
    ).toEqual({ name: undefined, language: "python3" });
    expect(
      workspaceDependenciesPathToLanguageAndFilename(`dependencies${sep}team.package.json`),
    ).toEqual({ name: "team", language: "bun" });
  }
  expect(isWorkspaceDependencies("f\\dependencies\\requirements.in")).toBe(false);
});
