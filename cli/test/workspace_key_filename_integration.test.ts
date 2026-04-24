import { describe, expect, test } from "bun:test";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import { execSync } from "node:child_process";
import os from "node:os";
import path from "node:path";
import { stringify as yamlStringify } from "yaml";

import { resolveWsNameForGitBranch } from "../src/core/specific_items.ts";
import { findResourceFile } from "../src/commands/script/script.ts";
import { resolveWsNameForConfigFromFlags } from "../src/commands/sync/sync.ts";
import type { SyncOptions } from "../src/core/conf.ts";

// Integration tests covering the bug where workspace-specific filenames used
// the raw git branch name instead of the wmill.yaml workspace config key.
// A per-item helper (findResourceFile) now resolves the effective wsName from
// wmill.yaml before falling back to the branch name.

async function withGitRepoAndConfig(
  config: unknown,
  branch: string,
  fn: (tempDir: string) => Promise<void>,
): Promise<void> {
  const tempDir = await mkdtemp(path.join(os.tmpdir(), "wmill_wskey_"));
  const originalCwd = process.cwd();
  try {
    // Set up a minimal git repo on the target branch so getCurrentGitBranch()
    // returns the expected value. An initial commit is needed so HEAD points
    // somewhere and `git rev-parse --abbrev-ref HEAD` succeeds.
    execSync(`git init -q -b ${branch}`, { cwd: tempDir });
    execSync(`git config user.email test@example.com`, { cwd: tempDir });
    execSync(`git config user.name test`, { cwd: tempDir });

    await writeFile(
      path.join(tempDir, "wmill.yaml"),
      yamlStringify(config),
    );

    execSync(`git add wmill.yaml && git commit -q -m init`, { cwd: tempDir });

    process.chdir(tempDir);
    await fn(tempDir);
  } finally {
    process.chdir(originalCwd);
    await rm(tempDir, { recursive: true, force: true });
  }
}

describe("resolveWsNameForGitBranch", () => {
  test("returns the wmill.yaml config key for a branch matched via gitBranch field", async () => {
    await withGitRepoAndConfig(
      {
        workspaces: {
          myKey: { gitBranch: "main", workspaceId: "prod" },
        },
      },
      "main",
      async () => {
        const wsName = await resolveWsNameForGitBranch("main");
        expect(wsName).toEqual("myKey");
      },
    );
  });

  test("returns the wmill.yaml config key when the branch equals the key and gitBranch is not set", async () => {
    await withGitRepoAndConfig(
      {
        workspaces: {
          staging: { workspaceId: "stg_workspace" },
        },
      },
      "staging",
      async () => {
        const wsName = await resolveWsNameForGitBranch("staging");
        expect(wsName).toEqual("staging");
      },
    );
  });

  test("falls back to the branch name when no matching workspace entry exists", async () => {
    await withGitRepoAndConfig(
      {
        workspaces: {
          production: { gitBranch: "main" },
        },
      },
      "feature-x",
      async () => {
        const wsName = await resolveWsNameForGitBranch("feature-x");
        expect(wsName).toEqual("feature-x");
      },
    );
  });
});

describe("resolveWsNameForConfigFromFlags", () => {
  test("--workspace matching a config key resolves, even with --base-url", () => {
    const opts: SyncOptions & { branch?: string; workspace?: string } = {
      workspace: "test",
      baseUrl: "http://127.0.0.1:8080/",
      workspaces: {
        test: { gitBranch: "main" },
        prod: { gitBranch: "main" },
      },
    } as any;
    expect(resolveWsNameForConfigFromFlags(opts)).toEqual("test");
  });

  test("--workspace not in config with --base-url returns undefined (treat as ad-hoc credential)", () => {
    const opts: SyncOptions & { branch?: string; workspace?: string } = {
      workspace: "adhocWorkspaceId",
      baseUrl: "https://other.windmill.dev/",
      workspaces: {
        test: { gitBranch: "main" },
      },
    } as any;
    expect(resolveWsNameForConfigFromFlags(opts)).toBeUndefined();
  });

  test("--workspace matching config key resolves even without --base-url", () => {
    const opts: SyncOptions & { branch?: string; workspace?: string } = {
      workspace: "prod",
      workspaces: { test: {}, prod: {} },
    } as any;
    expect(resolveWsNameForConfigFromFlags(opts)).toEqual("prod");
  });

  test("--branch takes precedence and looks up by gitBranch", () => {
    const opts: SyncOptions & { branch?: string; workspace?: string } = {
      branch: "main",
      workspace: "someOtherKey",
      workspaces: {
        test: { gitBranch: "main" },
        prod: { gitBranch: "release" },
      },
    } as any;
    expect(resolveWsNameForConfigFromFlags(opts)).toEqual("test");
  });

  test("no flags returns undefined", () => {
    const opts: SyncOptions & { branch?: string; workspace?: string } = {
      workspaces: { test: {} },
    } as any;
    expect(resolveWsNameForConfigFromFlags(opts)).toBeUndefined();
  });

  test("reserved key 'commonSpecificItems' is not accepted as a config key", () => {
    const opts: SyncOptions & { branch?: string; workspace?: string } = {
      workspace: "commonSpecificItems",
      workspaces: {
        test: {},
        commonSpecificItems: { variables: ["f/**"] } as any,
      },
    } as any;
    expect(resolveWsNameForConfigFromFlags(opts)).toBeUndefined();
  });
});

describe("findResourceFile picks the wsName-named file, not the branch-named file", () => {
  test("finds the workspace-specific resource file using the config key as suffix", async () => {
    await withGitRepoAndConfig(
      {
        workspaces: {
          myKey: { gitBranch: "main", workspaceId: "prod" },
        },
      },
      "main",
      async (tempDir) => {
        // Only the config-key-named file exists on disk. A wmill without this
        // fix would look for `f/foo.main.resource.yaml` (the branch name) and
        // either miss this file or pick a stale branch-named file.
        await mkdir(path.join(tempDir, "f"), { recursive: true });
        await writeFile(
          path.join(tempDir, "f/foo.myKey.resource.yaml"),
          "value: {}\nresource_type: text\n",
        );

        const found = await findResourceFile("f/foo.resource.file.txt");
        expect(found).toEqual("f/foo.myKey.resource.yaml");
      },
    );
  });
});
