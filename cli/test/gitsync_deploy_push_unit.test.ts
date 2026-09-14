import { expect, test } from "bun:test";
import { execFileSync } from "node:child_process";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { gitSyncDeployPush } from "../src/utils/git.ts";

function git(cwd: string, ...args: string[]): string {
  return execFileSync("git", args, { cwd, encoding: "utf8" }).trim();
}

// A seeded clone of a bare remote, with `files` committed on main.
async function seededClone(
  files: Record<string, string>,
): Promise<{ bare: string; work: string }> {
  const bare = await mkdtemp(join(tmpdir(), "wmill_deploy_push_bare_"));
  execFileSync("git", ["init", "--quiet", "--bare", "--initial-branch=main", bare]);
  const work = await mkdtemp(join(tmpdir(), "wmill_deploy_push_work_"));
  git(work, "init", "--quiet", "--initial-branch=main");
  git(work, "config", "user.email", "seed@windmill.dev");
  git(work, "config", "user.name", "seed");
  for (const [path, content] of Object.entries(files)) {
    await mkdir(join(work, path, ".."), { recursive: true });
    await writeFile(join(work, path), content);
  }
  git(work, "add", "-A");
  git(work, "commit", "--quiet", "-m", "seed");
  git(work, "remote", "add", "origin", `file://${bare}`);
  git(work, "push", "--quiet", "-u", "origin", "main");
  return { bare, work };
}

function deployPushIn(work: string, path: string) {
  const cwd = process.cwd();
  process.chdir(work);
  try {
    return gitSyncDeployPush({
      items: [{ path_type: "script", path, commit_msg: `deploy ${path}` }],
      authorName: "windmill",
      authorEmail: "windmill@windmill.dev",
    });
  } finally {
    process.chdir(cwd);
  }
}

test("a rewritten shared lockfile is committed with the deployed item", async () => {
  const { bare, work } = await seededClone({
    "wmill-lock.yaml": "locks: {}\n",
    "f/dd/a.script.yaml": "lock: '!inline locks/requirements.in.lock'\n",
    "locks/requirements.in.lock": "requests==2.31.0\n",
  });
  // What the deploy callback's pull leaves behind after `f/dd/a` was relocked:
  // the item's own files are unchanged, only the shared file moved.
  await writeFile(join(work, "locks/requirements.in.lock"), "requests==2.32.3\n");

  expect(deployPushIn(work, "f/dd/a").pushed).toBe(true);
  expect(git(work, "show", "--name-only", "--format=", "HEAD")).toBe(
    "locks/requirements.in.lock",
  );
  expect(git(bare, "cat-file", "-p", "main:locks/requirements.in.lock")).toBe(
    "requests==2.32.3",
  );

  await rm(bare, { recursive: true, force: true });
  await rm(work, { recursive: true, force: true });
});

test("a repository without shared lockfiles is left alone", async () => {
  const { bare, work } = await seededClone({
    "wmill-lock.yaml": "locks: {}\n",
    "f/dd/a.script.yaml": "lock: '!inline f/dd/a.script.lock'\n",
    "f/dd/a.script.lock": "requests==2.31.0\n",
  });

  expect(deployPushIn(work, "f/dd/a").pushed).toBe(false);
  expect(git(bare, "rev-parse", "main")).toBe(git(work, "rev-parse", "HEAD"));

  await rm(bare, { recursive: true, force: true });
  await rm(work, { recursive: true, force: true });
});
