/**
 * E2E regression test for git-sync promotion / `use_individual_branch`.
 *
 * Guards the hub/28229 regression: the git-sync deployment callback must push
 * each deployed object to a dedicated `wm_deploy/<workspace>/<...>` branch when
 * `use_individual_branch` is set — NOT straight to the cloned base branch
 * (e.g. a protected `main`, which fails with GH006).
 *
 * The CLI's `wmill sync pull --git-deploy-items ...` now owns that branch
 * checkout + commit + push (previously hub-script-only, hence untestable).
 * This drives it against a real local bare repo so the regression is caught
 * deterministically, with no network and no GitHub.
 */

import { expect, test } from "bun:test";
import { execFileSync } from "node:child_process";
import { mkdtemp, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { withTestBackend } from "./test_backend.ts";
import { shouldSkipOnCI } from "./cargo_backend.ts";
import { addWorkspace } from "../workspace.ts";

function git(cwd: string, ...args: string[]): string {
  return execFileSync("git", args, { cwd, encoding: "utf8" }).trim();
}

// Refs (branches) present on the bare remote.
function remoteBranches(bareDir: string): string[] {
  const out = execFileSync(
    "git",
    ["--git-dir", bareDir, "for-each-ref", "--format=%(refname)", "refs/heads/"],
    { encoding: "utf8" },
  ).trim();
  return out ? out.split("\n") : [];
}

function remoteHead(bareDir: string, branch: string): string {
  return execFileSync(
    "git",
    ["--git-dir", bareDir, "rev-parse", `refs/heads/${branch}`],
    { encoding: "utf8" },
  ).trim();
}

test.skipIf(shouldSkipOnCI())(
  "git-sync promotion: use_individual_branch pushes to wm_deploy branch, not main",
  async () => {
    await withTestBackend(async (backend, tempDir) => {
      const ws = backend.workspace; // "test"
      await addWorkspace(
        {
          remote: backend.baseUrl,
          workspaceId: ws,
          name: ws,
          token: backend.token,
        } as any,
        { force: true, configDir: backend.testConfigDir },
      );

      // --- 1. Local bare "remote" repo seeded with an initial `main` commit ---
      const bareDir = await mkdtemp(join(tmpdir(), "wmill_promo_bare_"));
      execFileSync("git", ["init", "--bare", "--initial-branch=main", bareDir]);
      const seedDir = await mkdtemp(join(tmpdir(), "wmill_promo_seed_"));
      git(seedDir, "init", "--initial-branch=main");
      git(seedDir, "config", "user.email", "seed@windmill.dev");
      git(seedDir, "config", "user.name", "seed");
      await writeFile(join(seedDir, "README.md"), "# promo test\n");
      git(seedDir, "add", "-A");
      git(seedDir, "commit", "-m", "seed");
      git(seedDir, "remote", "add", "origin", `file://${bareDir}`);
      git(seedDir, "push", "-u", "origin", "main");
      const seedMain = remoteHead(bareDir, "main");

      // --- 2. Workspace content to sync (a script in a folder) ---
      await backend.apiRequest!(`/api/w/${ws}/folders/create`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name: "promo", owners: [], extra_perms: {} }),
      });
      await backend.apiRequest!(`/api/w/${ws}/scripts/create`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          path: "f/promo/foo",
          summary: "",
          description: "",
          content: "export async function main() { return 1 }",
          language: "bun",
        }),
      });

      // --- 3. git_repository resource + git-sync config pointing at the bare repo ---
      await backend.apiRequest!(`/api/w/${ws}/resources/create`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          path: "u/test/promo_repo",
          resource_type: "git_repository",
          value: { url: `file://${bareDir}`, branch: "main", token: "" },
        }),
      });
      await backend.updateGitSyncConfig!({
        git_sync_settings: {
          repositories: [
            {
              git_repo_resource_path: "u/test/promo_repo",
              script_path: "f/**",
              use_individual_branch: true,
              group_by_folder: false,
              settings: {
                include_path: ["f/**"],
                include_type: ["script"],
              },
            },
          ],
        },
      });

      const deployItems = JSON.stringify([
        { path_type: "script", path: "f/promo/foo", commit_msg: "deploy foo" },
      ]);

      // --- Case A: use_individual_branch=true -> wm_deploy branch, main untouched ---
      const workA = await mkdtemp(join(tmpdir(), "wmill_promo_a_"));
      git(workA, "clone", `file://${bareDir}`, ".");
      await writeFile(
        join(workA, "wmill.yaml"),
        "defaultTs: bun\nincludes:\n  - f/**\nexcludes: []\n",
      );
      const resA = await backend.runCLICommand(
        [
          "sync",
          "pull",
          "--yes",
          "--skip-branch-validation",
          "--repository",
          "u/test/promo_repo",
          "--use-individual-branch",
          "--git-deploy-items",
          deployItems,
        ],
        workA,
      );
      expect(resA.code).toBe(0);

      const branchesA = remoteBranches(bareDir);
      const expectedBranch = `refs/heads/wm_deploy/${ws}/script/f__promo__foo`;
      expect(branchesA).toContain(expectedBranch);
      // The regression: main MUST be untouched (no direct push to protected base).
      expect(remoteHead(bareDir, "main")).toBe(seedMain);

      // --- Case B: workspace-wide (no flag) -> pushes to main, no wm_deploy ---
      const workB = await mkdtemp(join(tmpdir(), "wmill_promo_b_"));
      git(workB, "clone", `file://${bareDir}`, ".");
      await writeFile(
        join(workB, "wmill.yaml"),
        "defaultTs: bun\nincludes:\n  - f/**\nexcludes: []\n",
      );
      const resB = await backend.runCLICommand(
        [
          "sync",
          "pull",
          "--yes",
          "--skip-branch-validation",
          "--repository",
          "u/test/promo_repo",
          "--git-deploy-items",
          deployItems,
        ],
        workB,
      );
      expect(resB.code).toBe(0);
      expect(remoteHead(bareDir, "main")).not.toBe(seedMain);
      expect(
        remoteBranches(bareDir).filter((b) => b.includes("wm_deploy")).length,
      ).toBe(1); // only the one from Case A, none new

      await rm(bareDir, { recursive: true, force: true });
      await rm(seedDir, { recursive: true, force: true });
      await rm(workA, { recursive: true, force: true });
      await rm(workB, { recursive: true, force: true });
    });
  },
);
