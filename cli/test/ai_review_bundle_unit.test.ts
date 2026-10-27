/**
 * `wmill review` carries the frontend's AI review core, vendored by
 * cli/generate-ai-review.ts into src/commands/review/aiReviewCore.gen.ts.
 *
 * If that copy drifts, a pull request is reviewed with a different prompt, budget
 * or answer parsing than the deployment request panel uses. Fix by running
 * `bun run gen:ai-review` from cli/.
 *
 * No backend required.
 */

import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { buildAiReviewCore, OUT_FILE } from "../generate-ai-review.ts";
import { itemOf, numberDiff } from "../src/commands/review/review.ts";

describe("ai review core", () => {
  test("the committed copy matches the frontend source", () => {
    const lf = (s: string) => s.replace(/\r\n/g, "\n");
    expect(lf(readFileSync(OUT_FILE, "utf-8"))).toBe(lf(buildAiReviewCore()));
  });
});

describe("wmill review", () => {
  // Grouping decides which files the model reads as one item, the item a finding anchors to,
  // and which review instructions cover it.
  test("maps synced files to the workspace item they make up", () => {
    expect(
      [
        "f/a/flow.flow/flow.yaml",
        "f/a/flow__flow/step.inline_script.ts",
        "f/a/s.py",
        "f/a/s.script.yaml",
        "f/a/s__mod/helper.py",
        "f/a/sched.schedule.yaml",
        "f/a/folder.meta.yaml",
        "f/a/REVIEW.resource.file.md",
        "f/a/creds.variable.yaml",
        ".github/workflows/review.yml",
      ].map((f) => Object.values(itemOf(f)).join(":")),
    ).toEqual([
      "flow:f/a/flow",
      "flow:f/a/flow",
      "script:f/a/s",
      "script:f/a/s",
      "script:f/a/s",
      "schedule:f/a/sched",
      "folder:f/a",
      "resource:f/a/REVIEW",
      "variable:f/a/creds",
      "file:.github/workflows/review.yml",
    ]);
  });

  test("numbers each diff line by the side it is on", () => {
    const diff = [
      "diff --git a/x b/x",
      "@@ -10,3 +10,3 @@ fn",
      " keep",
      "-old",
      "+new",
      " tail",
    ].join("\n");
    const numbered = numberDiff(diff)!;
    expect(numbered.text.split("\n").slice(1)).toEqual([
      "R10  keep",
      "L11 -old",
      "R11 +new",
      "R12  tail",
    ]);
    expect([...numbered.left]).toEqual([11]);
    expect([...numbered.right]).toEqual([10, 11, 12]);
    expect(numberDiff("Binary files a/x and b/x differ")).toBeUndefined();
  });
});
