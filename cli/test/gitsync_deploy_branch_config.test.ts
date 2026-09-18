import { expect, test } from "bun:test";
import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { withTestBackend } from "./test_backend.ts";

function git(cwd: string, ...args: string[]): string {
  return execFileSync("git", args, { cwd, encoding: "utf8" }).trim();
}

// The git-sync deploy callback starts in a clone of the tracked branch and
// switches to the fork's branch before pulling. What it writes there must follow
// that branch's wmill.yaml: a fork branch that turned on `dedupeLockfiles` keeps
// its shared lockfile instead of getting a `.script.lock` per script back.
test("git-sync fork deploy follows the fork branch's wmill.yaml", async () => {
  await withTestBackend(async (backend) => {
    const post = (path: string, body: unknown) =>
      backend.apiRequest!(`/api/w/${backend.workspace}${path}`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });
    const created = await post("/workspace_dependencies/create", {
      workspace_id: backend.workspace,
      language: "python3",
      name: null,
      content: "wmill\n",
    });
    expect(created.ok).toBe(true);
    await post("/folders/create", { name: "dedupe" });
    const script = await post("/scripts/create", {
      path: "f/dedupe/a",
      summary: "",
      description: "",
      content: "def main():\n    pass\n",
      language: "python3",
      lock: "wmill==1.0.0\n",
    });
    expect(script.ok).toBe(true);

    const bare = await mkdtemp(join(tmpdir(), "wmill_deploy_cfg_bare_"));
    const seed = await mkdtemp(join(tmpdir(), "wmill_deploy_cfg_seed_"));
    const work = await mkdtemp(join(tmpdir(), "wmill_deploy_cfg_work_"));
    try {
      execFileSync("git", ["init", "--bare", "--initial-branch=main", bare]);
      git(seed, "init", "--initial-branch=main");
      git(seed, "config", "user.email", "seed@windmill.dev");
      git(seed, "config", "user.name", "seed");
      git(seed, "remote", "add", "origin", `file://${bare}`);
      const wmillYaml = "defaultTs: bun\nincludes:\n  - f/dedupe/**\nexcludes: []\n";
      await writeFile(join(seed, "wmill.yaml"), wmillYaml);
      git(seed, "add", "-A");
      git(seed, "commit", "-m", "main");
      git(seed, "push", "origin", "main");

      // Any parent id makes the callback deploy as a fork, to this branch.
      const forkBranch = `wm-fork/main/${backend.workspace}`;
      git(seed, "checkout", "-b", forkBranch);
      await writeFile(
        join(seed, "wmill.yaml"),
        wmillYaml + "dedupeLockfiles: true\n",
      );
      const pulled = await backend.runCLICommand(["sync", "pull", "--yes"], seed);
      expect(pulled.code).toBe(0);
      expect(existsSync(join(seed, "locks/requirements.in.lock"))).toBe(true);
      git(seed, "add", "-A");
      git(seed, "commit", "-m", "dedupe");
      git(seed, "push", "origin", forkBranch);

      git(work, "clone", `file://${bare}`, ".");
      const deployed = await backend.runCLICommand(
        [
          "sync",
          "git-deploy",
          "--repository",
          "u/test/unused",
          "--git-deploy-items",
          JSON.stringify([
            { path_type: "script", path: "f/dedupe/a", commit_msg: "deploy" },
          ]),
          "--parent-workspace-id",
          "parent",
        ],
        work,
      );
      expect(deployed.code).toBe(0);
      expect(git(work, "rev-parse", "--abbrev-ref", "HEAD")).toBe(forkBranch);
      expect(git(work, "status", "--porcelain")).toBe("");
    } finally {
      await rm(bare, { recursive: true, force: true });
      await rm(seed, { recursive: true, force: true });
      await rm(work, { recursive: true, force: true });
    }
  });
});
