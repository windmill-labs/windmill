/**
 * E2E regression test for git-sync promotion / `use_individual_branch`.
 *
 * Guards the hub/28229 regression: the git-sync deployment callback must push
 * each deployed object to a dedicated `wm_deploy/<workspace>/<...>` branch when
 * `use_individual_branch` is set — NOT straight to the cloned base branch
 * (e.g. a protected `main`, which fails with GH006).
 *
 * Contract: `wmill sync git-deploy` does branch checkout + pull only. Commit
 * + push are the caller's job — the hub script does them in the same process
 * as `set_gpg_signing_secret` so the GPG agent's passphrase cache is still
 * warm at sign time (WIN-1974). This test replicates the caller half (git
 * add + commit + push) inline so the full promotion regression stays caught.
 */

import { expect, test } from "bun:test";
import { execFileSync } from "node:child_process";
import { mkdtemp, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { withTestBackend } from "./test_backend.ts";
import { shouldSkipOnCI } from "./cargo_backend.ts";
import { addWorkspace } from "../workspace.ts";

// The HTTP-trigger promotion test creates an http_trigger, whose API routes are
// behind the `http_trigger` cargo feature — NOT in the default EE test feature
// set. The shared test backend reads TEST_FEATURES at construction (first
// `withTestBackend` call), so appending here at module load enables it. Guarded
// on shouldSkipOnCI() so we only widen the build when these EE tests actually
// run (i.e. EE_LICENSE_KEY present); minimal CI builds stay untouched.
if (!shouldSkipOnCI()) {
  process.env["TEST_FEATURES"] = [process.env["TEST_FEATURES"], "http_trigger"]
    .filter(Boolean)
    .join(",");
}

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

// True if `filePath` exists in the tree of `branch` on the bare remote.
function fileExistsOnBranch(
  bareDir: string,
  branch: string,
  filePath: string,
): boolean {
  try {
    execFileSync(
      "git",
      ["--git-dir", bareDir, "cat-file", "-e", `refs/heads/${branch}:${filePath}`],
      { stdio: "ignore" },
    );
    return true;
  } catch {
    return false;
  }
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

      // Caller-half: stage anything the CLI's pull dropped, commit on the
      // current branch (which the CLI just checked out), and push. Mirrors
      // what the hub script does in production after `wmill sync git-deploy`.
      const commitAndPush = (work: string) => {
        git(work, "config", "user.email", "test@windmill.dev");
        git(work, "config", "user.name", "test");
        git(work, "add", "-A");
        try {
          git(work, "diff", "--cached", "--quiet");
          // Exit 0 = nothing staged; nothing to commit. Still push the
          // (possibly new) branch ref so the assertions see it.
        } catch {
          git(work, "commit", "-m", "deploy foo");
        }
        git(work, "push", "--porcelain", "-u", "origin", "HEAD");
      };

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
          "git-deploy",
          "--repository",
          "u/test/promo_repo",
          "--use-individual-branch",
          "--git-deploy-items",
          deployItems,
        ],
        workA,
      );
      expect(resA.code).toBe(0);
      commitAndPush(workA);

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
          "git-deploy",
          "--repository",
          "u/test/promo_repo",
          "--git-deploy-items",
          deployItems,
        ],
        workB,
      );
      expect(resB.code).toBe(0);
      commitAndPush(workB);

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

/**
 * Regression test for the promotion trigger-include bug (fix/gitsync-promotion-
 * trigger-export): deploying a trigger (or any excluded-by-default kind:
 * schedule, group, user, settings, key) with `use_individual_branch` must still
 * land the object file on the `wm_deploy` branch.
 *
 * Root cause: `deriveGitSyncDeployIncludes` used to force the per-kind include
 * flags (`includeTriggers` etc.) to false in individual-branch mode. The
 * server-side tarball export STRIPS those object kinds entirely when their
 * include flag is false (`if include_triggers { … }` in workspaces_export.rs),
 * and `extraIncludes` is only a client-side filter over what the tarball
 * already contains — it can't recover a file the server never sent. So the
 * pull wrote no trigger file, the wm_deploy branch was created empty of the
 * trigger, and production's `git add '<path>**'` failed with "pathspec did not
 * match any files". A script (always-included kind) never hit this — hence the
 * dedicated trigger case here.
 *
 * Without the fix this test fails: the branch exists but the
 * `*.http_trigger.yaml` file is absent from it.
 */
test.skipIf(shouldSkipOnCI())(
  "git-sync promotion: use_individual_branch lands a trigger file on the wm_deploy branch",
  async () => {
    await withTestBackend(async (backend) => {
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

      // --- 1. Bare "remote" seeded with an initial `main` commit ---
      const bareDir = await mkdtemp(join(tmpdir(), "wmill_promo_trig_bare_"));
      execFileSync("git", ["init", "--bare", "--initial-branch=main", bareDir]);
      const seedDir = await mkdtemp(join(tmpdir(), "wmill_promo_trig_seed_"));
      git(seedDir, "init", "--initial-branch=main");
      git(seedDir, "config", "user.email", "seed@windmill.dev");
      git(seedDir, "config", "user.name", "seed");
      await writeFile(join(seedDir, "README.md"), "# promo trigger test\n");
      git(seedDir, "add", "-A");
      git(seedDir, "commit", "-m", "seed");
      git(seedDir, "remote", "add", "origin", `file://${bareDir}`);
      git(seedDir, "push", "-u", "origin", "main");
      const seedMain = remoteHead(bareDir, "main");

      // --- 2. Workspace content: a script + an HTTP trigger pointing at it ---
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
      const trigRes = await backend.apiRequest!(`/api/w/${ws}/http_triggers/create`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          path: "f/promo/hook",
          script_path: "f/promo/foo",
          route_path: "promo_hook",
          is_flow: false,
          http_method: "post",
          authentication_method: "none",
          is_static_website: false,
          request_type: "sync",
        }),
      });
      // Guard against the route silently 404ing (the http_trigger cargo feature
      // not being built) — otherwise the pull below would find nothing to sync
      // and the real assertion would fail with a confusing message.
      expect(trigRes.status).toBe(201);

      // --- 3. git_repository resource + git-sync config (triggers included) ---
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
                include_type: ["script", "trigger"],
              },
            },
          ],
        },
      });

      // The backend sets path_type "httptrigger" (no underscore) on the deploy
      // item — see DeployedObject::HttpTrigger => "httptrigger" in git_sync_ee.rs.
      const deployItems = JSON.stringify([
        {
          path_type: "httptrigger",
          path: "f/promo/hook",
          commit_msg: "deploy hook",
        },
      ]);

      const work = await mkdtemp(join(tmpdir(), "wmill_promo_trig_work_"));
      git(work, "clone", `file://${bareDir}`, ".");
      // Option B semantic: in promotion mode the deploy forces NOTHING — the
      // trigger lands only because THIS target's effective wmill.yaml opts
      // triggers in. (Reverting the source fix re-introduces the force-`false`
      // that clobbers this `includeTriggers: true`, so the file is dropped.)
      await writeFile(
        join(work, "wmill.yaml"),
        "defaultTs: bun\nincludes:\n  - f/**\nexcludes: []\nincludeTriggers: true\n",
      );
      const res = await backend.runCLICommand(
        [
          "sync",
          "git-deploy",
          "--repository",
          "u/test/promo_repo",
          "--use-individual-branch",
          "--git-deploy-items",
          deployItems,
        ],
        work,
      );
      expect(res.code).toBe(0);

      // Caller-half (mirrors the hub script): stage what the pull wrote, commit
      // on the checked-out wm_deploy branch, push.
      git(work, "config", "user.email", "test@windmill.dev");
      git(work, "config", "user.name", "test");
      git(work, "add", "-A");
      try {
        git(work, "diff", "--cached", "--quiet");
      } catch {
        git(work, "commit", "-m", "deploy hook");
      }
      git(work, "push", "--porcelain", "-u", "origin", "HEAD");

      const expectedBranch = `refs/heads/wm_deploy/${ws}/httptrigger/f__promo__hook`;
      expect(remoteBranches(bareDir)).toContain(expectedBranch);
      // The regression: the trigger file MUST be present on the branch. Without
      // the fix the include flag is false, the server strips the trigger from
      // the tarball, the pull writes nothing, and this file is absent.
      expect(
        fileExistsOnBranch(
          bareDir,
          `wm_deploy/${ws}/httptrigger/f__promo__hook`,
          "f/promo/hook.http_trigger.yaml",
        ),
      ).toBe(true);
      // Base branch untouched (individual-branch never pushes to the base).
      expect(remoteHead(bareDir, "main")).toBe(seedMain);

      await rm(bareDir, { recursive: true, force: true });
      await rm(seedDir, { recursive: true, force: true });
      await rm(work, { recursive: true, force: true });
    });
  },
);

/**
 * Same regression as the HTTP-trigger case above, for a `schedule` — a
 * different excluded-by-default kind that exercises a DISTINCT path: its own
 * include flag (`includeSchedules`), its own server-side `if include_schedules`
 * tarball-strip branch, and its own `.schedule.yaml` extension. Unlike triggers
 * it needs no extra cargo feature, so it guards the fix even where the
 * trigger-specific features aren't built.
 *
 * Without the fix this test fails: the branch exists but the
 * `*.schedule.yaml` file is absent from it.
 */
test.skipIf(shouldSkipOnCI())(
  "git-sync promotion: use_individual_branch lands a schedule file on the wm_deploy branch",
  async () => {
    await withTestBackend(async (backend) => {
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

      // --- 1. Bare "remote" seeded with an initial `main` commit ---
      const bareDir = await mkdtemp(join(tmpdir(), "wmill_promo_sched_bare_"));
      execFileSync("git", ["init", "--bare", "--initial-branch=main", bareDir]);
      const seedDir = await mkdtemp(join(tmpdir(), "wmill_promo_sched_seed_"));
      git(seedDir, "init", "--initial-branch=main");
      git(seedDir, "config", "user.email", "seed@windmill.dev");
      git(seedDir, "config", "user.name", "seed");
      await writeFile(join(seedDir, "README.md"), "# promo schedule test\n");
      git(seedDir, "add", "-A");
      git(seedDir, "commit", "-m", "seed");
      git(seedDir, "remote", "add", "origin", `file://${bareDir}`);
      git(seedDir, "push", "-u", "origin", "main");
      const seedMain = remoteHead(bareDir, "main");

      // --- 2. Workspace content: a script + a (disabled) schedule for it ---
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
      const schedRes = await backend.apiRequest!(`/api/w/${ws}/schedules/create`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          path: "f/promo/sched",
          schedule: "0 0 12 * * *",
          timezone: "UTC",
          script_path: "f/promo/foo",
          is_flow: false,
          args: {},
          enabled: false,
        }),
      });
      expect(schedRes.status).toBe(200);

      // --- 3. git_repository resource + git-sync config (schedules included) ---
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
                include_type: ["script", "schedule"],
              },
            },
          ],
        },
      });

      const deployItems = JSON.stringify([
        {
          path_type: "schedule",
          path: "f/promo/sched",
          commit_msg: "deploy sched",
        },
      ]);

      const work = await mkdtemp(join(tmpdir(), "wmill_promo_sched_work_"));
      git(work, "clone", `file://${bareDir}`, ".");
      // Option B semantic: in promotion mode the deploy forces NOTHING — the
      // schedule lands only because THIS target's effective wmill.yaml opts
      // schedules in. (Reverting the source fix re-introduces the force-`false`
      // that clobbers this `includeSchedules: true`, so the file is dropped.)
      await writeFile(
        join(work, "wmill.yaml"),
        "defaultTs: bun\nincludes:\n  - f/**\nexcludes: []\nincludeSchedules: true\n",
      );
      const res = await backend.runCLICommand(
        [
          "sync",
          "git-deploy",
          "--repository",
          "u/test/promo_repo",
          "--use-individual-branch",
          "--git-deploy-items",
          deployItems,
        ],
        work,
      );
      expect(res.code).toBe(0);

      // Caller-half (mirrors the hub script): stage what the pull wrote, commit
      // on the checked-out wm_deploy branch, push.
      git(work, "config", "user.email", "test@windmill.dev");
      git(work, "config", "user.name", "test");
      git(work, "add", "-A");
      try {
        git(work, "diff", "--cached", "--quiet");
      } catch {
        git(work, "commit", "-m", "deploy sched");
      }
      git(work, "push", "--porcelain", "-u", "origin", "HEAD");

      const expectedBranch = `refs/heads/wm_deploy/${ws}/schedule/f__promo__sched`;
      expect(remoteBranches(bareDir)).toContain(expectedBranch);
      // The regression: the schedule file MUST be present on the branch. Without
      // the fix the include flag is false, the server strips the schedule from
      // the tarball, the pull writes nothing, and this file is absent.
      expect(
        fileExistsOnBranch(
          bareDir,
          `wm_deploy/${ws}/schedule/f__promo__sched`,
          "f/promo/sched.schedule.yaml",
        ),
      ).toBe(true);
      // Base branch untouched (individual-branch never pushes to the base).
      expect(remoteHead(bareDir, "main")).toBe(seedMain);

      await rm(bareDir, { recursive: true, force: true });
      await rm(seedDir, { recursive: true, force: true });
      await rm(work, { recursive: true, force: true });
    });
  },
);

/**
 * Regression test for WIN-2052: a script with companion modules
 * (workflows-as-code) is stored on disk under a `__mod/` folder
 * (`<path>__mod/script.ts`, `<path>__mod/script.yaml`, `<path>__mod/<module>`)
 * — NOT under the dotted `<path>.script.*` layout a module-less script uses.
 *
 * Root cause: `gitSyncIncludePattern`'s script (default) case returned only
 * `<path>.*`, which the `ignoreF` extra-includes filter (minimatch) does NOT
 * match against any `__mod/` file (literal dot, no dot after the name). So the
 * pull filtered out every module file, the wm_deploy branch was created with
 * nothing staged, and production's `git add '<path>**'` failed with "pathspec
 * did not match any files". The fix adds `<path>__mod/**` to the pattern.
 *
 * The nested `utils/math.ts` module guards the `**` (vs `*`) choice: a single
 * star would not match files in module subdirectories.
 *
 * Without the fix this test fails: the branch exists but the `__mod/` files
 * (entry point + modules) are absent from it.
 */
test.skipIf(shouldSkipOnCI())(
  "git-sync promotion: use_individual_branch lands a module script's __mod/ files on the wm_deploy branch",
  async () => {
    await withTestBackend(async (backend) => {
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

      // --- 1. Bare "remote" seeded with an initial `main` commit ---
      const bareDir = await mkdtemp(join(tmpdir(), "wmill_promo_mod_bare_"));
      execFileSync("git", ["init", "--bare", "--initial-branch=main", bareDir]);
      const seedDir = await mkdtemp(join(tmpdir(), "wmill_promo_mod_seed_"));
      git(seedDir, "init", "--initial-branch=main");
      git(seedDir, "config", "user.email", "seed@windmill.dev");
      git(seedDir, "config", "user.name", "seed");
      await writeFile(join(seedDir, "README.md"), "# promo module test\n");
      git(seedDir, "add", "-A");
      git(seedDir, "commit", "-m", "seed");
      git(seedDir, "remote", "add", "origin", `file://${bareDir}`);
      git(seedDir, "push", "-u", "origin", "main");
      const seedMain = remoteHead(bareDir, "main");

      // --- 2. Workspace content: a script WITH companion modules (one flat,
      //        one nested) — this is what triggers the `__mod/` folder layout. ---
      await backend.apiRequest!(`/api/w/${ws}/folders/create`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name: "promo", owners: [], extra_perms: {} }),
      });
      const scriptRes = await backend.apiRequest!(`/api/w/${ws}/scripts/create`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          path: "f/promo/mod_script",
          summary: "",
          description: "",
          content: "export async function main() { return 1 }",
          language: "bun",
          modules: {
            "helper.ts": {
              content: "export function help() { return 2 }\n",
              language: "bun",
            },
            "utils/math.ts": {
              content: "export function add(a: number, b: number) { return a + b }\n",
              language: "bun",
            },
          },
        }),
      });
      expect(scriptRes.status).toBe(201);

      // --- 3. git_repository resource + git-sync config (scripts included) ---
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

      // A script always has path_type "script" — modules are a property of the
      // script, not a distinct object kind — so this hits gitSyncIncludePattern's
      // default case, the one the fix amends.
      const deployItems = JSON.stringify([
        {
          path_type: "script",
          path: "f/promo/mod_script",
          commit_msg: "deploy mod_script",
        },
      ]);

      const work = await mkdtemp(join(tmpdir(), "wmill_promo_mod_work_"));
      git(work, "clone", `file://${bareDir}`, ".");
      await writeFile(
        join(work, "wmill.yaml"),
        "defaultTs: bun\nincludes:\n  - f/**\nexcludes: []\n",
      );
      const res = await backend.runCLICommand(
        [
          "sync",
          "git-deploy",
          "--repository",
          "u/test/promo_repo",
          "--use-individual-branch",
          "--git-deploy-items",
          deployItems,
        ],
        work,
      );
      expect(res.code).toBe(0);

      // Caller-half (mirrors the hub script): stage what the pull wrote, commit
      // on the checked-out wm_deploy branch, push.
      git(work, "config", "user.email", "test@windmill.dev");
      git(work, "config", "user.name", "test");
      git(work, "add", "-A");
      try {
        git(work, "diff", "--cached", "--quiet");
      } catch {
        git(work, "commit", "-m", "deploy mod_script");
      }
      git(work, "push", "--porcelain", "-u", "origin", "HEAD");

      const branch = `wm_deploy/${ws}/script/f__promo__mod_script`;
      expect(remoteBranches(bareDir)).toContain(`refs/heads/${branch}`);
      // The regression: every `__mod/` file MUST be present on the branch — the
      // entry point, the flat module, and the nested module. Without the fix the
      // extra-includes pattern matches none of them, the pull writes nothing, and
      // these files are absent (and `git add` would have failed in production).
      for (const f of [
        "f/promo/mod_script__mod/script.ts",
        "f/promo/mod_script__mod/helper.ts",
        "f/promo/mod_script__mod/utils/math.ts",
      ]) {
        expect(fileExistsOnBranch(bareDir, branch, f)).toBe(true);
      }
      // Base branch untouched (individual-branch never pushes to the base).
      expect(remoteHead(bareDir, "main")).toBe(seedMain);

      await rm(bareDir, { recursive: true, force: true });
      await rm(seedDir, { recursive: true, force: true });
      await rm(work, { recursive: true, force: true });
    });
  },
);

/**
 * Regression test for WIN-1997: forking a workspace with git sync configured
 * must publish a `wm-fork/<branch>/<id>` branch to the remote.
 *
 * The fork-branch callback runs the sync script with `only_create_branch:
 * true` and no items. The hub script delegates branch checkout + push of that
 * empty ref to `wmill sync git-deploy --only-create-branch` — its own
 * in-process commit+push runs ONLY for the `!only_create_branch` path. So if
 * the CLI doesn't push the freshly checked-out branch here, nothing does and
 * the fork branch never reaches the remote (the symptom that broke the e2e
 * test after #9284 moved commit+push to the caller). This guards that the CLI
 * owns the push for the branch-only case.
 */
test.skipIf(shouldSkipOnCI())(
  "git-sync fork: only_create_branch publishes the wm-fork branch (CLI owns the push)",
  async () => {
    await withTestBackend(async (backend) => {
      // Bare "remote" seeded with an initial `main` commit.
      const bareDir = await mkdtemp(join(tmpdir(), "wmill_fork_bare_"));
      execFileSync("git", ["init", "--bare", "--initial-branch=main", bareDir]);
      const seedDir = await mkdtemp(join(tmpdir(), "wmill_fork_seed_"));
      git(seedDir, "init", "--initial-branch=main");
      git(seedDir, "config", "user.email", "seed@windmill.dev");
      git(seedDir, "config", "user.name", "seed");
      await writeFile(join(seedDir, "README.md"), "# fork test\n");
      git(seedDir, "add", "-A");
      git(seedDir, "commit", "-m", "seed");
      git(seedDir, "remote", "add", "origin", `file://${bareDir}`);
      git(seedDir, "push", "-u", "origin", "main");
      const seedMain = remoteHead(bareDir, "main");

      // The CWD the hub script runs git-deploy in: a clone of the repo on main.
      const work = await mkdtemp(join(tmpdir(), "wmill_fork_work_"));
      git(work, "clone", `file://${bareDir}`, ".");
      await writeFile(
        join(work, "wmill.yaml"),
        "defaultTs: bun\nincludes:\n  - f/**\nexcludes: []\n",
      );

      // Branch creation happens BEFORE the fork workspace exists (step 1 of the
      // fork flow), so we pass the fork workspace id straight through — whoami
      // returns synthetic superadmin info for it. No items, only_create_branch.
      const forkWs = "wm-fork-clitest";
      const res = await backend.runCLICommand(
        [
          "sync",
          "git-deploy",
          "--repository",
          "u/test/unused_on_branch_only_path",
          "--git-deploy-items",
          "[]",
          "--only-create-branch",
        ],
        work,
        { workspace: forkWs },
      );
      expect(res.code).toBe(0);

      // The regression: with NO caller-side commit/push, the fork branch must
      // already be on the remote because the CLI pushed it.
      expect(remoteBranches(bareDir)).toContain("refs/heads/wm-fork/main/clitest");
      // Base branch untouched — branch-only publish creates no commit.
      expect(remoteHead(bareDir, "main")).toBe(seedMain);

      await rm(bareDir, { recursive: true, force: true });
      await rm(seedDir, { recursive: true, force: true });
      await rm(work, { recursive: true, force: true });
    });
  },
);
