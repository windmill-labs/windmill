/**
 * Workspace role overrides sync through settings.yaml: `sync pull` writes them, `sync push`
 * applies them, and an override of a role the instance does not define fails the push.
 */

import { expect, test } from "bun:test";
import { writeFile, readFile } from "node:fs/promises";
import { parse, stringify } from "yaml";
import { withTestBackend } from "./test_backend.ts";
import { shouldSkipOnCI } from "./cargo_backend.ts";
import { addWorkspace } from "../workspace.ts";

const WMILL_YAML = `defaultTs: bun
includes:
  - "**"
includeSettings: true`;

async function setup(backend: any, tempDir: string, name: string) {
  await addWorkspace(
    { remote: backend.baseUrl, workspaceId: backend.workspace, name, token: backend.token },
    { force: true, configDir: backend.testConfigDir },
  );
  await writeFile(`${tempDir}/wmill.yaml`, WMILL_YAML, "utf-8");
}

async function setRoleOverrides(backend: any, overrides: unknown) {
  const resp = await backend.apiRequest!(
    `/api/w/${backend.workspace}/workspaces/role_overrides`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(overrides),
    },
  );
  if (!resp.ok) throw new Error(`role_overrides: ${resp.status} ${await resp.text()}`);
}

async function storedRoles(backend: any) {
  const resp = await backend.apiRequest!(`/api/w/${backend.workspace}/workspaces/roles`);
  expect(resp.ok).toBe(true);
  return (await resp.json()) as {
    overrides: Record<string, unknown>;
    roles: { name: string; label: string; source: string; capabilities: any }[];
  };
}

test.skipIf(shouldSkipOnCI())("Role overrides: pulled into settings.yaml and pushed back", async () => {
  await withTestBackend(async (backend, tempDir) => {
    await setup(backend, tempDir, "role_overrides_roundtrip");
    await setRoleOverrides(backend, {
      operator: { label: "Runner", capabilities: { flows: "compose" } },
      auditor: { label: "Auditor", capabilities: { code: false } },
    });

    const pull = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);
    expect(pull.code).toEqual(0);
    const settings = parse(await readFile(`${tempDir}/settings.yaml`, "utf-8")) as any;
    expect(settings.role_overrides).toEqual({
      operator: { label: "Runner", capabilities: { flows: "compose" } },
    });
    expect(settings.workspace_only_roles).toEqual({
      auditor: { label: "Auditor", capabilities: { code: false } },
    });

    // Edit both kinds in the file: override developer too, swap the workspace-only role.
    settings.role_overrides.developer = { capabilities: { fork: false } };
    settings.workspace_only_roles = {
      scripter: { label: "Scripter", capabilities: { code: true, flows: "none", apps: "none" } },
    };
    await writeFile(`${tempDir}/settings.yaml`, stringify(settings), "utf-8");

    const push = await backend.runCLICommand(["sync", "push", "--yes"], tempDir);
    expect(push.code).toEqual(0);

    const roles = await storedRoles(backend);
    expect(roles.overrides).toEqual({
      operator: { label: "Runner", capabilities: { flows: "compose" } },
      developer: { capabilities: { fork: false } },
      scripter: { label: "Scripter", capabilities: { code: true, flows: "none", apps: "none" } },
    });
    const scripter = roles.roles.find((r) => r.name === "scripter");
    expect(scripter?.source).toEqual("workspace");
    expect(roles.roles.some((r) => r.name === "auditor")).toBe(false);

    // Pulling again writes the file that was pushed, so a synced repo does not churn.
    const before = await readFile(`${tempDir}/settings.yaml`, "utf-8");
    const repull = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);
    expect(repull.code).toEqual(0);
    const after = parse(await readFile(`${tempDir}/settings.yaml`, "utf-8")) as any;
    expect(after.role_overrides).toEqual(parse(before).role_overrides);
    expect(after.workspace_only_roles).toEqual(parse(before).workspace_only_roles);
  });
});

test.skipIf(shouldSkipOnCI())("Role overrides: an override of an undefined instance role fails the push", async () => {
  await withTestBackend(async (backend, tempDir) => {
    await setup(backend, tempDir, "role_overrides_missing_role");
    await setRoleOverrides(backend, { operator: { capabilities: { flows: "compose" } } });

    const pull = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);
    expect(pull.code).toEqual(0);
    const settings = parse(await readFile(`${tempDir}/settings.yaml`, "utf-8")) as any;
    settings.role_overrides.reviewer = { capabilities: { code: false } };
    settings.role_overrides.operator = { capabilities: { flows: "none" } };
    await writeFile(`${tempDir}/settings.yaml`, stringify(settings), "utf-8");

    const push = await backend.runCLICommand(["sync", "push", "--yes"], tempDir);
    expect(push.code).not.toEqual(0);
    expect(push.stdout + push.stderr).toContain("`reviewer`");
    expect(push.stdout + push.stderr).toContain("does not define");

    const roles = await storedRoles(backend);
    expect(roles.overrides).toEqual({ operator: { capabilities: { flows: "compose" } } });
  });
});

test.skipIf(shouldSkipOnCI())("Role overrides: a settings.yaml without the keys leaves them alone", async () => {
  await withTestBackend(async (backend, tempDir) => {
    await setup(backend, tempDir, "role_overrides_unmanaged");
    await setRoleOverrides(backend, {
      operator: { capabilities: { flows: "compose" } },
      auditor: { label: "Auditor", capabilities: {} },
    });

    const pull = await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);
    expect(pull.code).toEqual(0);
    const settings = parse(await readFile(`${tempDir}/settings.yaml`, "utf-8")) as any;
    delete settings.role_overrides;
    delete settings.workspace_only_roles;
    settings.webhook = "https://example.com/hook";
    await writeFile(`${tempDir}/settings.yaml`, stringify(settings), "utf-8");

    const push = await backend.runCLICommand(["sync", "push", "--yes"], tempDir);
    expect(push.code).toEqual(0);
    const roles = await storedRoles(backend);
    expect(roles.overrides).toEqual({
      operator: { capabilities: { flows: "compose" } },
      auditor: { label: "Auditor", capabilities: {} },
    });
  });
});
