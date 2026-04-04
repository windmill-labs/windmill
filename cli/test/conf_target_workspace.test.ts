import { expect, test } from "bun:test";
import { getEffectiveSettings, type SyncOptions } from "../src/core/conf.ts";

// =============================================================================
// CONF.TS TARGET WORKSPACE TESTS
// Tests for getEffectiveSettings with targetWorkspace parameter
// Verifies that defaultPermissionedAs is resolved based on the target workspace
// =============================================================================

const ROOT_RULES = [
  { email: "root@example.com", path_pattern: "**" },
];

const STAGING_RULES = [
  { email: "staging-deployer@example.com", path_pattern: "f/staging/**" },
];

const PROD_RULES = [
  { email: "prod-deployer@example.com", path_pattern: "f/prod/**" },
];

function makeConfig(opts?: { rootRules?: boolean }): SyncOptions {
  return {
    defaultTs: "bun",
    includes: ["f/**"],
    defaultPermissionedAs: opts?.rootRules !== false ? ROOT_RULES : undefined,
    gitBranches: {
      staging: {
        workspaceId: "staging-ws",
        baseUrl: "https://staging.windmill.dev/",
        overrides: {},
        defaultPermissionedAs: STAGING_RULES,
      },
      production: {
        workspaceId: "prod-ws",
        baseUrl: "https://prod.windmill.dev/",
        overrides: {},
        defaultPermissionedAs: PROD_RULES,
      },
    },
  };
}

test("targetWorkspace: uses matching branch's defaultPermissionedAs", async () => {
  const config = makeConfig();

  const settings = await getEffectiveSettings(
    config, undefined, true, true, "staging",
    { workspaceId: "staging-ws", remote: "https://staging.windmill.dev/" },
  );
  expect(settings.defaultPermissionedAs).toEqual(STAGING_RULES);
});

test("targetWorkspace: uses correct branch when workspace differs from git branch", async () => {
  const config = makeConfig();

  // On staging branch, but targeting production workspace
  const settings = await getEffectiveSettings(
    config, undefined, true, true, "staging",
    { workspaceId: "prod-ws", remote: "https://prod.windmill.dev/" },
  );
  expect(settings.defaultPermissionedAs).toEqual(PROD_RULES);
});

test("targetWorkspace: falls back to root-level rules when no branch matches workspace", async () => {
  const config = makeConfig();

  const settings = await getEffectiveSettings(
    config, undefined, true, true, "staging",
    { workspaceId: "unknown-ws", remote: "https://other.windmill.dev/" },
  );
  expect(settings.defaultPermissionedAs).toEqual(ROOT_RULES);
});

test("targetWorkspace: falls back to root-level when matched branch has no rules", async () => {
  const config: SyncOptions = {
    defaultTs: "bun",
    defaultPermissionedAs: ROOT_RULES,
    gitBranches: {
      staging: {
        workspaceId: "staging-ws",
        baseUrl: "https://staging.windmill.dev/",
        overrides: {},
        // No defaultPermissionedAs defined
      },
    },
  };

  const settings = await getEffectiveSettings(
    config, undefined, true, true, "staging",
    { workspaceId: "staging-ws", remote: "https://staging.windmill.dev/" },
  );
  expect(settings.defaultPermissionedAs).toEqual(ROOT_RULES);
});

test("targetWorkspace: requires both workspaceId AND baseUrl to match", async () => {
  const config = makeConfig();

  // Same workspaceId but different baseUrl — should NOT match staging branch
  const settings = await getEffectiveSettings(
    config, undefined, true, true, "staging",
    { workspaceId: "staging-ws", remote: "https://other.windmill.dev/" },
  );
  expect(settings.defaultPermissionedAs).toEqual(ROOT_RULES);
});

test("targetWorkspace: without targetWorkspace, uses branch-level rules as before", async () => {
  const config = makeConfig();

  // No targetWorkspace — should use the branch's rules (existing behavior)
  const settings = await getEffectiveSettings(
    config, undefined, true, true, "staging",
  );
  expect(settings.defaultPermissionedAs).toEqual(STAGING_RULES);
});

test("targetWorkspace: without gitBranches, targetWorkspace is ignored", async () => {
  const config: SyncOptions = {
    defaultTs: "bun",
    defaultPermissionedAs: ROOT_RULES,
  };

  const settings = await getEffectiveSettings(
    config, undefined, true, true, undefined,
    { workspaceId: "any-ws", remote: "https://any.windmill.dev/" },
  );
  expect(settings.defaultPermissionedAs).toEqual(ROOT_RULES);
});

test("targetWorkspace: no root-level rules and no match returns undefined", async () => {
  const config: SyncOptions = {
    defaultTs: "bun",
    gitBranches: {
      staging: {
        workspaceId: "staging-ws",
        baseUrl: "https://staging.windmill.dev/",
        overrides: {},
        defaultPermissionedAs: STAGING_RULES,
      },
    },
  };

  const settings = await getEffectiveSettings(
    config, undefined, true, true, undefined,
    { workspaceId: "unknown-ws", remote: "https://other.windmill.dev/" },
  );
  expect(settings.defaultPermissionedAs).toBeUndefined();
});
