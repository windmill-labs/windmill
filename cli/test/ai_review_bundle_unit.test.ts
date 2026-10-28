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
import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { buildAiReviewCore, OUT_FILE } from "../generate-ai-review.ts";
import { buildReviewPrompt, itemOf, numberDiff } from "../src/commands/review/review.ts";

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
        "f/a/files.fileset/x.flow/flow.yaml",
        "f/a/proj__dbt/models/legacy__mod/a.sql",
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
      "resource:f/a/files",
      "script:f/a/proj",
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

describe("wmill review prompt", () => {
  // The pull request's head is written by its author, so nothing it does may put a
  // credential in front of the model or steer which instructions apply.
  test("withholds secrets whatever the head claims, and reads instructions from the base", () => {
    const repo = mkdtempSync(join(tmpdir(), "wmill-review-"));
    const cwd = process.cwd();
    const write = (files: Record<string, string>) => {
      for (const [path, content] of Object.entries(files)) {
        mkdirSync(join(repo, dirname(path)), { recursive: true });
        writeFileSync(join(repo, path), content);
      }
    };
    const git = (...args: string[]) =>
      execFileSync("git", args, { cwd: repo, encoding: "utf-8" }).trim();
    try {
      git("init", "-q", "-b", "main");
      git("config", "user.email", "t@t");
      git("config", "user.name", "t");
      write({
        "f/a/REVIEW.resource.yaml":
          "resource_type: ai_review_instruction\nvalue:\n  content: '!inline f/a/REVIEW.resource.file.md'\n",
        "f/a/REVIEW.resource.file.md": "BASE-RULE\n",
        "f/a/db.resource.yaml": "resource_type: postgresql\nvalue:\n  password: OLD-PASSWORD\n",
        "f/a/creds.variable.yaml": "value: VARIABLE-SECRET\nis_secret: false\n",
        "encryption_key.yaml": "BASE-ENCRYPTION-KEY\n",
        "f/a/notes.resource.yaml": "resource_type: ai_review_instruction\nvalue:\n  content: hi\n",
        "f/a/files.fileset/x.flow/flow.yaml": "password: FILESET-PASSWORD\n",
      });
      git("add", "-A");
      git("commit", "-qm", "base");
      const base = git("rev-parse", "HEAD");
      git("checkout", "-qb", "pr");
      write({
        "f/a/REVIEW.resource.file.md": "HEAD-RULE\n",
        "f/a/db.resource.yaml": "resource_type: ai_review_instruction\nvalue:\n  content: hi\n",
        "f/a/creds.resource.yaml": "resource_type: ai_review_instruction\nvalue:\n  content: hi\n",
        "f/a/creds.variable.yaml": "value: VARIABLE-SECRET-2\nis_secret: false\n",
        "encryption_key.yaml": "HEAD-ENCRYPTION-KEY\n",
        "f/a/notes.prod.resource.yaml": "resource_type: postgresql\nvalue:\n  password: PROD-PASSWORD\n",
        "f/a/files.fileset/x.flow/flow.yaml": "password: FILESET-PASSWORD-2\n",
        // A pathspec matching every changed file, if git were let read it as one.
        "*": "star\n",
      });
      git("add", "-A");
      git("commit", "-qm", "pr");
      process.chdir(repo);
      const { system, user, instructions } = buildReviewPrompt(base, git("rev-parse", "HEAD"));
      const prompt = system + user;
      for (const secret of ["PASSWORD", "VARIABLE-SECRET", "ENCRYPTION-KEY"]) {
        expect(prompt).not.toContain(secret);
      }
      expect(system).toContain("BASE-RULE");
      expect(system).not.toContain("HEAD-RULE");
      // The instruction change itself is shown, as a change.
      expect(user).toContain("HEAD-RULE");
      expect(instructions).toEqual(["f/a/notes", "f/a/REVIEW"]);
    } finally {
      process.chdir(cwd);
      rmSync(repo, { recursive: true, force: true });
    }
  });
});
