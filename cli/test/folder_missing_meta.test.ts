/**
 * Tests for missing folder.meta.yaml detection during sync push,
 * the `folder add-missing` command, and the simplified `folder push` command.
 */

import { expect, test, describe } from "bun:test";
import { writeFile, mkdir, readFile, rm, mkdtemp } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { getTestBackend, createNonAdminUser } from "./test_backend.ts";

type IsolatedWorkspaceTestContext = {
  backend: any;
  tempDir: string;
  workspaceId: string;
  runCLICommand: (
    args: string[],
    opts?: { token?: string }
  ) => Promise<{ stdout: string; stderr: string; code: number }>;
  apiRequest: (path: string, options?: RequestInit) => Promise<Response>;
};

async function createWorkspace(backend: any, workspaceId: string): Promise<void> {
  const response = await backend.apiRequest!("/api/workspaces/create", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      id: workspaceId,
      // Workspace name has a 50-char DB limit; keep it identical to the short ID.
      name: workspaceId,
    }),
  });

  if (!response.ok) {
    const error = await response.text();
    if (!error.includes("already exists") && !error.includes("duplicate")) {
      throw new Error(`Failed to create workspace ${workspaceId}: ${error}`);
    }
    return;
  }
  await response.text();
}

async function withIsolatedWorkspace(
  testFn: (ctx: IsolatedWorkspaceTestContext) => Promise<void>
): Promise<void> {
  const backend = await getTestBackend();
  const tempDir = await mkdtemp(join(tmpdir(), "windmill_cli_test_"));
  const workspaceId = `fmeta_${Date.now().toString(36)}_${Math.random()
    .toString(36)
    .slice(2, 6)}`;
  let workspaceCreated = false;

  try {
    await createWorkspace(backend, workspaceId);
    workspaceCreated = true;

    await testFn({
      backend,
      tempDir,
      workspaceId,
      runCLICommand: (args: string[], opts?: { token?: string }) =>
        backend.runCLICommand(args, tempDir, {
          workspace: workspaceId,
          token: opts?.token,
        }),
      apiRequest: (path: string, options?: RequestInit) =>
        backend.apiRequest!(`/api/w/${workspaceId}${path}`, options),
    });
  } finally {
    if (workspaceCreated) {
      try {
        const archiveResponse = await backend.apiRequest!(
          `/api/w/${workspaceId}/workspaces/archive`,
          { method: "POST" }
        );
        await archiveResponse.text();
      } catch {
        // Best-effort cleanup to avoid exceeding non-enterprise workspace limits.
      }
    }
    await rm(tempDir, { recursive: true, force: true });
  }
}

function wmillYaml(): string {
  return `defaultTs: bun\nincludes:\n  - "**"\nexcludes: []\n`;
}

// =============================================================================
// folder new — creates folder.meta.yaml with summary and display_name
// =============================================================================

describe("folder new", () => {
  test("creates folder.meta.yaml with summary and display_name", async () => {
    await withIsolatedWorkspace(async ({ tempDir, runCLICommand }) => {
      const folderName = `newfolder${Date.now()}`;
      const result = await runCLICommand(
        ["folder", "new", folderName, "--summary", "My summary"],
      );

      expect(result.code).toEqual(0);

      const metaPath = join(tempDir, "f", folderName, "folder.meta.yaml");
      const content = await readFile(metaPath, "utf-8");
      expect(content).toContain("summary: My summary");
      expect(content).toContain(`display_name: ${folderName}`);
      expect(content).toContain("owners:");
      expect(content).toContain("extra_perms:");
    });
  });

  test("creates folder.meta.yaml with empty summary when none provided", async () => {
    await withIsolatedWorkspace(async ({ tempDir, runCLICommand }) => {
      const folderName = `nosummary${Date.now()}`;
      const result = await runCLICommand(
        ["folder", "new", folderName],
      );

      expect(result.code).toEqual(0);

      const content = await readFile(
        join(tempDir, "f", folderName, "folder.meta.yaml"),
        "utf-8"
      );
      expect(content).toContain('summary: ""');
      expect(content).toContain(`display_name: ${folderName}`);
    });
  });

  test("fails if folder.meta.yaml already exists", async () => {
    await withIsolatedWorkspace(async ({ runCLICommand }) => {
      const folderName = `dupfolder${Date.now()}`;
      // Create first
      await runCLICommand(["folder", "new", folderName]);

      // Try again — should fail
      const result = await runCLICommand(
        ["folder", "new", folderName],
      );
      expect(result.code).not.toEqual(0);
    });
  });
});

// =============================================================================
// folder add-missing — scaffolds missing folder.meta.yaml files
// =============================================================================

describe("folder add-missing", () => {
  test("creates folder.meta.yaml for directories missing one", async () => {
    await withIsolatedWorkspace(async ({ tempDir, runCLICommand }) => {
      // Create two folders: one with meta, one without
      const withMeta = `withmeta${Date.now()}`;
      const withoutMeta = `withoutmeta${Date.now()}`;

      await mkdir(join(tempDir, "f", withMeta), { recursive: true });
      await writeFile(
        join(tempDir, "f", withMeta, "folder.meta.yaml"),
        'summary: ""\ndisplay_name: existing\nowners: []\nextra_perms: {}\n',
        "utf-8"
      );

      await mkdir(join(tempDir, "f", withoutMeta), { recursive: true });
      // No folder.meta.yaml for withoutMeta

      const result = await runCLICommand(
        ["folder", "add-missing", "-y"],
      );

      expect(result.code).toEqual(0);

      // withoutMeta should now have a folder.meta.yaml
      const createdMeta = await readFile(
        join(tempDir, "f", withoutMeta, "folder.meta.yaml"),
        "utf-8"
      );
      expect(createdMeta).toContain(`display_name: ${withoutMeta}`);
      expect(createdMeta).toContain("owners:");

      // withMeta should be unchanged
      const existingMeta = await readFile(
        join(tempDir, "f", withMeta, "folder.meta.yaml"),
        "utf-8"
      );
      expect(existingMeta).toContain("display_name: existing");
    });
  });

  test("reports nothing to do when all folders have meta", async () => {
    await withIsolatedWorkspace(async ({ tempDir, runCLICommand }) => {
      const folderName = `alldone${Date.now()}`;
      await mkdir(join(tempDir, "f", folderName), { recursive: true });
      await writeFile(
        join(tempDir, "f", folderName, "folder.meta.yaml"),
        'summary: ""\ndisplay_name: done\nowners: []\nextra_perms: {}\n',
        "utf-8"
      );

      const result = await runCLICommand(
        ["folder", "add-missing", "-y"],
      );

      expect(result.code).toEqual(0);
      expect(result.stdout + result.stderr).toContain("Nothing to do");
    });
  });

  test("reports nothing to do when no f/ directory exists", async () => {
    await withIsolatedWorkspace(async ({ runCLICommand }) => {
      const result = await runCLICommand(
        ["folder", "add-missing", "-y"],
      );

      expect(result.code).toEqual(0);
      expect(result.stdout + result.stderr).toContain("Nothing to do");
    });
  });
});

// =============================================================================
// folder push — simplified single-arg signature
// =============================================================================

describe("folder push", () => {
  test("pushes a folder by name", async () => {
    await withIsolatedWorkspace(async ({ tempDir, runCLICommand, apiRequest }) => {
      const folderName = `pushbyname${Date.now()}`;

      // Create local folder meta
      await mkdir(join(tempDir, "f", folderName), { recursive: true });
      await writeFile(
        join(tempDir, "f", folderName, "folder.meta.yaml"),
        `summary: "pushed"\ndisplay_name: "${folderName}"\nowners:\n  - "admin@windmill.dev"\nextra_perms: {}\n`,
        "utf-8"
      );

      const result = await runCLICommand(
        ["folder", "push", folderName],
      );

      expect(result.code).toEqual(0);
      expect(result.stdout + result.stderr).toContain("Folder pushed");

      // Verify via API
      const apiResp = await apiRequest(`/folders/get/${folderName}`);
      expect(apiResp.status).toEqual(200);
    });
  });

  test("fails when folder does not exist locally", async () => {
    await withIsolatedWorkspace(async ({ runCLICommand }) => {
      const result = await runCLICommand(
        ["folder", "push", "nonexistent"],
      );

      expect(result.code).not.toEqual(0);
    });
  });
});

// =============================================================================
// sync push — missing folder.meta.yaml detection
// =============================================================================

describe("sync push missing folder detection", () => {
  test("admin user gets warning but push succeeds", async () => {
    await withIsolatedWorkspace(async ({ tempDir, runCLICommand }) => {
      const uniqueId = Date.now();
      const folderName = `nometaadmin${uniqueId}`;

      await writeFile(join(tempDir, "wmill.yaml"), wmillYaml(), "utf-8");

      // Create a script inside a folder WITHOUT folder.meta.yaml
      await mkdir(join(tempDir, "f", folderName), { recursive: true });
      await writeFile(
        join(tempDir, "f", folderName, "test_script.ts"),
        'export async function main() { return "hello"; }',
        "utf-8"
      );

      const result = await runCLICommand(
        ["sync", "push", "--yes", "--includes", `f/${folderName}/**`],
      );

      // Admin should get a warning but push succeeds (exit 0)
      expect(result.code).toEqual(0);
      const output = result.stdout + result.stderr;
      expect(output).toContain("Missing folder.meta.yaml");
      expect(output).toContain(folderName);
      expect(output).toContain("wmill folder add-missing");
    });
  });

  test("no warning when folder.meta.yaml exists", async () => {
    await withIsolatedWorkspace(async ({ tempDir, runCLICommand }) => {
      const uniqueId = Date.now();
      const folderName = `withmeta${uniqueId}`;

      await writeFile(join(tempDir, "wmill.yaml"), wmillYaml(), "utf-8");

      // Create folder WITH folder.meta.yaml
      await mkdir(join(tempDir, "f", folderName), { recursive: true });
      await writeFile(
        join(tempDir, "f", folderName, "folder.meta.yaml"),
        `summary: ""\ndisplay_name: "${folderName}"\nowners: []\nextra_perms: {}\n`,
        "utf-8"
      );
      await writeFile(
        join(tempDir, "f", folderName, "test_script.ts"),
        'export async function main() { return "hello"; }',
        "utf-8"
      );

      const result = await runCLICommand(
        ["sync", "push", "--yes", "--includes", `f/${folderName}/**`],
      );

      expect(result.code).toEqual(0);
      const output = result.stdout + result.stderr;
      expect(output).not.toContain("Missing folder.meta.yaml");
    });
  });

  test.skipIf(!process.env["EE_LICENSE_KEY"])("non-admin user gets error and exit code 1", async () => {
    await withIsolatedWorkspace(async ({ backend, tempDir, workspaceId, runCLICommand, apiRequest }) => {
      const nonAdminToken = await createNonAdminUser(backend, workspaceId);

      const uniqueId = Date.now();
      const folderName = `nometanonadmin${uniqueId}`;

      await writeFile(join(tempDir, "wmill.yaml"), wmillYaml(), "utf-8");

      // Create a script inside a folder WITHOUT folder.meta.yaml
      // First create the folder on remote so the non-admin has somewhere to push
      await apiRequest(
        "/folders/create",
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            name: folderName,
            extra_perms: { "g/all": true },
          }),
        }
      );

      await mkdir(join(tempDir, "f", folderName), { recursive: true });
      await writeFile(
        join(tempDir, "f", folderName, "test_script.ts"),
        'export async function main() { return "hello"; }',
        "utf-8"
      );

      const result = await runCLICommand(
        ["sync", "push", "--yes", "--includes", `f/${folderName}/**`],
        { token: nonAdminToken }
      );

      // Non-admin should get exit code 1
      expect(result.code).toEqual(1);
      const output = result.stdout + result.stderr;
      expect(output).toContain("Missing folder.meta.yaml");
      expect(output).toContain("wmill folder add-missing");
    });
  });

  test("no warning for deleted changes without folder.meta.yaml", async () => {
    await withIsolatedWorkspace(async ({ tempDir, runCLICommand, apiRequest }) => {
      const uniqueId = Date.now();
      const folderName = `delfolder${uniqueId}`;

      // Create folder and script on remote via API
      await apiRequest(
        "/folders/create",
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ name: folderName }),
        }
      );

      await writeFile(join(tempDir, "wmill.yaml"), wmillYaml(), "utf-8");

      // Pull to get remote state, then delete the folder locally
      await runCLICommand(["sync", "pull", "--yes"]);

      // Remove the folder locally to trigger a "deleted" change
      await rm(join(tempDir, "f", folderName), { recursive: true, force: true });

      const result = await runCLICommand(
        ["sync", "push", "--yes", "--includes", `f/${folderName}/**`],
      );

      // Should not warn about missing meta for deleted items
      const output = result.stdout + result.stderr;
      expect(output).not.toContain("Missing folder.meta.yaml");
    });
  });
});
