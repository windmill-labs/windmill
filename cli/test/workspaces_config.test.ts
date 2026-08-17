import { expect, test, describe } from "bun:test";
import { writeFile, mkdir, readFile } from "node:fs/promises";
import { join } from "node:path";
import { withTestBackend } from "./test_backend.ts";
import { addWorkspace } from "../workspace.ts";
import { parseJsonFromCLIOutput } from "./test_config_helpers.ts";
import { getEffectiveSettings, readConfigFile, convertGitBranchesToWorkspaces, findWorkspaceByGitBranch, getEffectiveWorkspaceId, getEffectiveGitBranch, getWorkspaceNames, type SyncOptions } from "../src/core/conf.ts";
import { getSpecificItemsForCurrentBranch } from "../src/core/specific_items.ts";

// =============================================================================
// Helper
// =============================================================================

async function setupWorkspaceProfile(backend: any, workspaceName: string): Promise<void> {
  await addWorkspace({
    remote: backend.baseUrl,
    workspaceId: backend.workspace,
    name: workspaceName,
    token: backend.token,
  }, { force: true, configDir: backend.testConfigDir });
}

// =============================================================================
// UNIT TESTS: workspaces config helpers
// =============================================================================

describe("findWorkspaceByGitBranch", () => {
  test("finds workspace by key when gitBranch not set (default)", () => {
    const ws = { staging: { baseUrl: "https://staging.wm.dev" } };
    const match = findWorkspaceByGitBranch(ws as any, "staging");
    expect(match).toBeDefined();
    expect(match![0]).toEqual("staging");
  });

  test("finds workspace by explicit gitBranch", () => {
    const ws = { production: { gitBranch: "main", baseUrl: "https://app.wm.dev" } };
    const match = findWorkspaceByGitBranch(ws as any, "main");
    expect(match).toBeDefined();
    expect(match![0]).toEqual("production");
  });

  test("does not find workspace when gitBranch differs", () => {
    const ws = { production: { gitBranch: "main", baseUrl: "https://app.wm.dev" } };
    const match = findWorkspaceByGitBranch(ws as any, "production");
    expect(match).toBeUndefined();
  });

  test("returns undefined for undefined workspaces", () => {
    expect(findWorkspaceByGitBranch(undefined, "main")).toBeUndefined();
  });

  test("skips commonSpecificItems", () => {
    const ws = { commonSpecificItems: { variables: ["f/**"] }, staging: { baseUrl: "x" } };
    const match = findWorkspaceByGitBranch(ws as any, "commonSpecificItems");
    expect(match).toBeUndefined();
  });

  test("returns first match when multiple map to same gitBranch", () => {
    const ws = {
      alpha: { gitBranch: "develop", baseUrl: "a" },
      beta: { gitBranch: "develop", baseUrl: "b" },
    };
    const match = findWorkspaceByGitBranch(ws as any, "develop");
    expect(match![0]).toEqual("alpha");
  });
});

describe("getEffectiveWorkspaceId", () => {
  test("returns workspaceId when set", () => {
    expect(getEffectiveWorkspaceId("prod", { workspaceId: "production" })).toEqual("production");
  });

  test("defaults to workspace name when workspaceId not set", () => {
    expect(getEffectiveWorkspaceId("staging", {})).toEqual("staging");
  });
});

describe("getEffectiveGitBranch", () => {
  test("returns gitBranch when set", () => {
    expect(getEffectiveGitBranch("production", { gitBranch: "main" })).toEqual("main");
  });

  test("defaults to workspace name when gitBranch not set", () => {
    expect(getEffectiveGitBranch("staging", {})).toEqual("staging");
  });
});

describe("getWorkspaceNames", () => {
  test("excludes commonSpecificItems", () => {
    const ws = { staging: {}, production: {}, commonSpecificItems: { variables: [] } };
    expect(getWorkspaceNames(ws as any)).toEqual(["staging", "production"]);
  });

  test("returns empty for undefined", () => {
    expect(getWorkspaceNames(undefined)).toEqual([]);
  });
});

// =============================================================================
// UNIT TESTS: convertGitBranchesToWorkspaces
// =============================================================================

describe("convertGitBranchesToWorkspaces", () => {
  test("preserves all fields from old branch entries", () => {
    const old = {
      main: {
        baseUrl: "https://app.wm.dev",
        workspaceId: "production",
        overrides: { skipSecrets: false },
        promotionOverrides: { skipSecrets: true },
        specificItems: { variables: ["f/**"] },
        stateful: true,
        message: "auto",
      },
    };
    const ws = convertGitBranchesToWorkspaces(old as any);
    const entry = (ws as any).main;
    expect(entry.baseUrl).toEqual("https://app.wm.dev");
    expect(entry.workspaceId).toEqual("production");
    expect(entry.overrides).toEqual({ skipSecrets: false });
    expect(entry.promotionOverrides).toEqual({ skipSecrets: true });
    expect(entry.specificItems).toEqual({ variables: ["f/**"] });
    expect(entry.stateful).toEqual(true);
    expect(entry.message).toEqual("auto");
  });

  test("preserves commonSpecificItems", () => {
    const old = {
      commonSpecificItems: { variables: ["f/shared/**"], settings: true },
      main: { overrides: {} },
    };
    const ws = convertGitBranchesToWorkspaces(old as any);
    expect(ws.commonSpecificItems).toEqual({ variables: ["f/shared/**"], settings: true });
  });

  test("handles empty entries", () => {
    const ws = convertGitBranchesToWorkspaces({ main: {} } as any);
    expect((ws as any).main).toEqual({});
  });

  test("does not set gitBranch (defaults to key name)", () => {
    const ws = convertGitBranchesToWorkspaces({ main: { baseUrl: "x" } } as any);
    expect((ws as any).main.gitBranch).toBeUndefined();
  });
});

// =============================================================================
// UNIT TESTS: getEffectiveSettings with workspaces config
// =============================================================================

describe("getEffectiveSettings with workspaces", () => {
  test("applies overrides by workspace name", async () => {
    const config: SyncOptions = {
      includes: ["f/**"],
      workspaces: {
        staging: { overrides: { includes: ["staging/**"], skipVariables: true } },
        production: { overrides: { skipSecrets: false } },
      },
    };
    const s = await getEffectiveSettings(config, undefined, true, true, "staging");
    expect(s.includes).toEqual(["staging/**"]);
    expect(s.skipVariables).toEqual(true);
  });

  test("returns top-level settings for unknown workspace", async () => {
    const config: SyncOptions = { includes: ["f/**"], workspaces: { staging: { overrides: {} } } };
    const s = await getEffectiveSettings(config, undefined, true, true, "nonexistent");
    expect(s.includes).toEqual(["f/**"]);
  });

  test("promotion resolves by gitBranch", async () => {
    const config: SyncOptions = {
      workspaces: {
        production: {
          gitBranch: "main",
          promotionOverrides: { skipSecrets: false },
        },
      },
    };
    // --promotion main should find workspace "production" via gitBranch match
    const s = await getEffectiveSettings(config, "main", true, true);
    expect(s.skipSecrets).toEqual(false);
  });
});

// =============================================================================
// UNIT TESTS: getSpecificItemsForCurrentBranch with workspaces
// =============================================================================

describe("getSpecificItemsForCurrentBranch with workspaces", () => {
  test("looks up by workspace name override", () => {
    const config: SyncOptions = {
      workspaces: {
        staging: { specificItems: { variables: ["f/staging/**"] } },
        production: { specificItems: { variables: ["f/prod/**"] } },
        commonSpecificItems: { resources: ["f/shared/**"] },
      },
    };
    const items = getSpecificItemsForCurrentBranch(config, "staging");
    expect(items?.variables).toEqual(["f/staging/**"]);
    expect(items?.resources).toEqual(["f/shared/**"]);
  });

  test("returns undefined for unknown workspace", () => {
    const config: SyncOptions = {
      workspaces: { staging: { specificItems: { variables: ["f/**"] } } },
    };
    const items = getSpecificItemsForCurrentBranch(config, "nonexistent");
    expect(items).toBeUndefined();
  });

  test("returns undefined when no workspaces config", () => {
    expect(getSpecificItemsForCurrentBranch({}, "staging")).toBeUndefined();
  });

  test("merges common and workspace-specific items", () => {
    const config: SyncOptions = {
      workspaces: {
        commonSpecificItems: { variables: ["common/**"], resources: ["shared/**"] },
        dev: { specificItems: { variables: ["dev/**"], triggers: ["dev/triggers/**"] } },
      },
    };
    const items = getSpecificItemsForCurrentBranch(config, "dev");
    expect(items?.variables).toEqual(["common/**", "dev/**"]);
    expect(items?.resources).toEqual(["shared/**"]);
    expect(items?.triggers).toEqual(["dev/triggers/**"]);
  });
});

// =============================================================================
// INTEGRATION TESTS: new workspaces config format
// =============================================================================

test("Integration: workspaces config with --branch applies correct overrides", async () => {
  await withTestBackend(async (backend, tempDir) => {
    await setupWorkspaceProfile(backend, "ws_test");

    // New workspaces config with workspace name ≠ gitBranch
    await writeFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
skipVariables: false

workspaces:
  production:
    gitBranch: prod_branch
    overrides:
      skipVariables: true
  staging:
    overrides:
      skipVariables: false`, "utf-8");

    // --branch prod_branch should resolve to workspace "production" and skip variables
    const result = await backend.runCLICommand([
      'sync', 'pull',
      '--branch', 'prod_branch',
      '--dry-run',
      '--json-output'
    ], tempDir);

    expect(result.code).toEqual(0);
    const output = parseJsonFromCLIOutput(result.stdout);
    const paths = (output.changes || []).map((c: any) => c.path);
    const hasVariables = paths.some((p: string) => p.includes('.variable.yaml'));
    expect(hasVariables).toEqual(false);
  });
});

test("Integration: workspaces config with --branch resolves default gitBranch", async () => {
  await withTestBackend(async (backend, tempDir) => {
    await setupWorkspaceProfile(backend, "ws_test");

    // Workspace name = "staging", gitBranch defaults to "staging"
    await writeFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
skipVariables: false

workspaces:
  staging:
    overrides:
      skipVariables: true`, "utf-8");

    // --branch staging resolves to workspace "staging"
    const result = await backend.runCLICommand([
      'sync', 'pull',
      '--branch', 'staging',
      '--dry-run',
      '--json-output'
    ], tempDir);

    expect(result.code).toEqual(0);
    const output = parseJsonFromCLIOutput(result.stdout);
    const paths = (output.changes || []).map((c: any) => c.path);
    const hasVariables = paths.some((p: string) => p.includes('.variable.yaml'));
    expect(hasVariables).toEqual(false);
  });
});

test("Integration: different workspaces have different overrides", async () => {
  await withTestBackend(async (backend, tempDir) => {
    await setupWorkspaceProfile(backend, "ws_test");

    await writeFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
skipVariables: false
skipResources: false

workspaces:
  production:
    gitBranch: prod
    overrides:
      skipVariables: true
      skipResources: true
  development:
    gitBranch: dev
    overrides:
      skipVariables: false
      skipResources: false`, "utf-8");

    // prod should skip both
    const prodResult = await backend.runCLICommand([
      'sync', 'pull', '--branch', 'prod', '--dry-run', '--json-output'
    ], tempDir);
    expect(prodResult.code).toEqual(0);
    const prodPaths = (parseJsonFromCLIOutput(prodResult.stdout).changes || []).map((c: any) => c.path);
    expect(prodPaths.some((p: string) => p.includes('.variable.yaml'))).toEqual(false);
    expect(prodPaths.some((p: string) => p.includes('.resource.yaml'))).toEqual(false);

    // dev should include both
    const devResult = await backend.runCLICommand([
      'sync', 'pull', '--branch', 'dev', '--dry-run', '--json-output'
    ], tempDir);
    expect(devResult.code).toEqual(0);
    const devPaths = (parseJsonFromCLIOutput(devResult.stdout).changes || []).map((c: any) => c.path);
    expect(devPaths.some((p: string) => p.includes('.variable.yaml'))).toEqual(true);
    expect(devPaths.some((p: string) => p.includes('.resource.yaml'))).toEqual(true);
  });
});

test("Integration: legacy gitBranches config still works via normalization", async () => {
  await withTestBackend(async (backend, tempDir) => {
    await setupWorkspaceProfile(backend, "legacy_test");

    // Old gitBranches format on disk
    await writeFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
skipVariables: false

gitBranches:
  legacy_branch:
    overrides:
      skipVariables: true`, "utf-8");

    const result = await backend.runCLICommand([
      'sync', 'pull', '--branch', 'legacy_branch', '--dry-run', '--json-output'
    ], tempDir);
    expect(result.code).toEqual(0);
    const paths = (parseJsonFromCLIOutput(result.stdout).changes || []).map((c: any) => c.path);
    expect(paths.some((p: string) => p.includes('.variable.yaml'))).toEqual(false);
  });
});

test("Integration: legacy environments config still works via normalization", async () => {
  await withTestBackend(async (backend, tempDir) => {
    await setupWorkspaceProfile(backend, "env_test");

    // Old environments format on disk
    await writeFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
skipVariables: false

environments:
  env_branch:
    overrides:
      skipVariables: true`, "utf-8");

    const result = await backend.runCLICommand([
      'sync', 'pull', '--branch', 'env_branch', '--dry-run', '--json-output'
    ], tempDir);
    expect(result.code).toEqual(0);
    const paths = (parseJsonFromCLIOutput(result.stdout).changes || []).map((c: any) => c.path);
    expect(paths.some((p: string) => p.includes('.variable.yaml'))).toEqual(false);
  });
});

test("Integration: workspace-specific files use workspace name as suffix", async () => {
  await withTestBackend(async (backend, tempDir) => {
    await setupWorkspaceProfile(backend, "specific_test");

    // Workspace "production" with gitBranch "main" and specificItems
    await writeFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"

workspaces:
  production:
    gitBranch: prod_branch
    specificItems:
      variables:
        - "f/**"`, "utf-8");

    // Pull with --branch prod_branch → resolves to workspace "production"
    // workspace-specific files should use "production" as suffix (the workspace name)
    const result = await backend.runCLICommand([
      'sync', 'pull', '--branch', 'prod_branch', '--dry-run', '--json-output'
    ], tempDir);
    expect(result.code).toEqual(0);

    const output = parseJsonFromCLIOutput(result.stdout);
    const changes = output.changes || [];

    // Check that workspace-specific paths use the workspace name "production" as suffix
    const wsSpecificPaths = changes
      .filter((c: any) => c.workspace_specific_path)
      .map((c: any) => c.workspace_specific_path);

    for (const p of wsSpecificPaths) {
      expect(p).toContain(".production.");
      expect(p).not.toContain(".prod_branch.");
    }
  });
});

test("Integration: promotion with workspaces config", async () => {
  await withTestBackend(async (backend, tempDir) => {
    await setupWorkspaceProfile(backend, "promo_test");

    await writeFile(`${tempDir}/wmill.yaml`, `defaultTs: bun
includes:
  - "**"
skipVariables: false

workspaces:
  staging:
    gitBranch: staging_branch
    overrides:
      skipVariables: false
    promotionOverrides:
      skipVariables: true`, "utf-8");

    // --promotion staging_branch should find workspace "staging" via gitBranch
    // and apply promotionOverrides (skipVariables: true)
    const result = await backend.runCLICommand([
      'sync', 'pull',
      '--branch', 'staging_branch',
      '--promotion', 'staging_branch',
      '--dry-run',
      '--json-output'
    ], tempDir);
    expect(result.code).toEqual(0);
    const paths = (parseJsonFromCLIOutput(result.stdout).changes || []).map((c: any) => c.path);
    expect(paths.some((p: string) => p.includes('.variable.yaml'))).toEqual(false);
  });
});

test("Integration: config migrate converts gitBranches to workspaces", async () => {
  await withTestBackend(async (backend, tempDir) => {
    await setupWorkspaceProfile(backend, "migrate_test");

    // Write old format
    const yamlPath = `${tempDir}/wmill.yaml`;
    await writeFile(yamlPath, `defaultTs: bun
includes:
  - "f/**"

gitBranches:
  main:
    baseUrl: https://app.windmill.dev
    workspaceId: production
    overrides:
      skipSecrets: false
  staging:
    baseUrl: https://staging.windmill.dev
    overrides:
      includeSchedules: true
  commonSpecificItems:
    variables:
      - "f/shared/**"`, "utf-8");

    // Run migrate
    const result = await backend.runCLICommand([
      'config', 'migrate',
    ], tempDir);
    expect(result.code).toEqual(0);

    // Read back and verify
    const migrated = await readFile(yamlPath, "utf-8");
    expect(migrated).toContain("workspaces:");
    expect(migrated).not.toContain("gitBranches:");
    expect(migrated).toContain("production");
    expect(migrated).toContain("staging");
    expect(migrated).toContain("commonSpecificItems");
    expect(migrated).toContain("f/shared/**");
  });
});

test("Integration: config migrate is idempotent", async () => {
  await withTestBackend(async (backend, tempDir) => {
    await setupWorkspaceProfile(backend, "idempotent_test");

    const yamlPath = `${tempDir}/wmill.yaml`;
    await writeFile(yamlPath, `defaultTs: bun
workspaces:
  staging:
    baseUrl: https://staging.wm.dev
    overrides: {}`, "utf-8");

    const result = await backend.runCLICommand(['config', 'migrate'], tempDir);
    expect(result.code).toEqual(0);

    // File should be unchanged
    const content = await readFile(yamlPath, "utf-8");
    expect(content).toContain("workspaces:");
    expect(content).not.toContain("gitBranches:");
  });
});
