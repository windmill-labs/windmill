/**
 * A raw app's `value.files` keys are app-author-controlled and written back to
 * disk relative to the app folder on `wmill sync pull`. A key that resolves
 * outside the app folder (a `..` segment) must be rejected so a pulled file
 * always stays within its own app's folder.
 */

import { expect, test } from "bun:test";
import { join as joinPath } from "node:path";
import { rawAppFilePathWithinFolder } from "../src/commands/sync/sync.ts";

const APP = joinPath("u", "admin", "myapp.raw_app");

test("legit keys resolve to a path inside the app folder", () => {
  expect(rawAppFilePathWithinFolder(APP, "/index.tsx")).toBe(
    joinPath(APP, "index.tsx"),
  );
  expect(rawAppFilePathWithinFolder(APP, "/src/util.ts")).toBe(
    joinPath(APP, "src", "util.ts"),
  );
});

test("keys that resolve outside the app folder are rejected", () => {
  for (const key of [
    "/../sibling.ts", // into the app's own parent folder
    "/../../../f/other/outside.ts", // into an unrelated folder tree
    "/../../../../../../elsewhere.txt", // above the app folder entirely
    "/src/../../escape.ts",
  ]) {
    expect(() => rawAppFilePathWithinFolder(APP, key)).toThrow(
      /escapes the app folder/,
    );
  }
});
