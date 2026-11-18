import { expect, test } from "bun:test";
import { join as joinPath } from "node:path";
import { rawAppPathWithinFolder } from "../src/commands/sync/sync.ts";

const APP = joinPath("u", "admin", "myapp.raw_app");
const BACKEND = joinPath(APP, "wm_backend");

test("keys that stay inside the folder resolve to a path within it", () => {
  // `value.files` keys arrive with a leading slash the caller strips.
  expect(rawAppPathWithinFolder(APP, "index.tsx")).toBe(
    joinPath(APP, "index.tsx"),
  );
  expect(rawAppPathWithinFolder(APP, "src/util.ts")).toBe(
    joinPath(APP, "src", "util.ts"),
  );
  // A runnable id names its yaml under the backend folder.
  expect(rawAppPathWithinFolder(BACKEND, "a.yaml")).toBe(
    joinPath(BACKEND, "a.yaml"),
  );
});

test("keys that resolve outside the folder are rejected", () => {
  for (const [base, rel] of [
    [APP, "../sibling.ts"], // a files key into the app's parent folder
    [APP, "../../../f/other/outside.ts"], // into an unrelated folder tree
    [APP, "../../../../../../elsewhere.txt"], // above the app folder entirely
    [BACKEND, "../../../../etc/evil.yaml"], // a runnable id escaping the backend folder
  ] as const) {
    expect(() => rawAppPathWithinFolder(base, rel)).toThrow(
      /escapes the app folder/,
    );
  }
});
