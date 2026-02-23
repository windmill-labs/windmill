/**
 * Tests for missing folder.meta.yaml detection during sync push,
 * the `folder add-missing` command, and the simplified `folder push` command.
 */

import { expect, test, describe } from "bun:test";
import { writeFile, mkdir, readFile, stat, rm } from "node:fs/promises";
import { join } from "node:path";
import { withTestBackend, createNonAdminUser } from "./test_backend.ts";
import { addWorkspace } from "../workspace.ts";

async function setupWorkspaceProfile(backend: any): Promise<void> {
  await addWorkspace(
    {
      remote: backend.baseUrl,
      workspaceId: backend.workspace,
      name: "localhost_test_folder",
      token: backend.token,
    },
    { force: true, configDir: backend.testConfigDir }
  );
}

function wmillYaml(): string {
  return `defaultTs: bun\nincludes:\n  - "**"\nexcludes: []\n`;
}

// =============================================================================
// folder new — creates folder.meta.yaml with summary and display_name
// =============================================================================

describe("folder new", () => {
  test("creates folder.meta.yaml with summary and display_name", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const folderName = `newfolder${Date.now()}`;
      const result = await backend.runCLICommand(
        ["folder", "new", folderName, "--summary", "My summary"],
        tempDir
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
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const folderName = `nosummary${Date.now()}`;
      const result = await backend.runCLICommand(
        ["folder", "new", folderName],
        tempDir
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
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const folderName = `dupfolder${Date.now()}`;
      // Create first
      await backend.runCLICommand(["folder", "new", folderName], tempDir);

      // Try again — should fail
      const result = await backend.runCLICommand(
        ["folder", "new", folderName],
        tempDir
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
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

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

      const result = await backend.runCLICommand(
        ["folder", "add-missing", "-y"],
        tempDir
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
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const folderName = `alldone${Date.now()}`;
      await mkdir(join(tempDir, "f", folderName), { recursive: true });
      await writeFile(
        join(tempDir, "f", folderName, "folder.meta.yaml"),
        'summary: ""\ndisplay_name: done\nowners: []\nextra_perms: {}\n',
        "utf-8"
      );

      const result = await backend.runCLICommand(
        ["folder", "add-missing", "-y"],
        tempDir
      );

      expect(result.code).toEqual(0);
      expect(result.stdout + result.stderr).toContain("Nothing to do");
    });
  });

  test("reports nothing to do when no f/ directory exists", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const result = await backend.runCLICommand(
        ["folder", "add-missing", "-y"],
        tempDir
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
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const folderName = `pushbyname${Date.now()}`;

      // Create local folder meta
      await mkdir(join(tempDir, "f", folderName), { recursive: true });
      await writeFile(
        join(tempDir, "f", folderName, "folder.meta.yaml"),
        `summary: "pushed"\ndisplay_name: "${folderName}"\nowners:\n  - "admin@windmill.dev"\nextra_perms: {}\n`,
        "utf-8"
      );

      const result = await backend.runCLICommand(
        ["folder", "push", folderName],
        tempDir
      );

      expect(result.code).toEqual(0);
      expect(result.stdout + result.stderr).toContain("Folder pushed");

      // Verify via API
      const apiResp = await backend.apiRequest!(
        `/api/w/${backend.workspace}/folders/get/${folderName}`
      );
      expect(apiResp.status).toEqual(200);
    });
  });

  test("fails when folder does not exist locally", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const result = await backend.runCLICommand(
        ["folder", "push", "nonexistent"],
        tempDir
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
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

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

      const result = await backend.runCLICommand(
        ["sync", "push", "--yes", "--includes", `f/${folderName}/**`],
        tempDir
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
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

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

      const result = await backend.runCLICommand(
        ["sync", "push", "--yes", "--includes", `f/${folderName}/**`],
        tempDir
      );

      expect(result.code).toEqual(0);
      const output = result.stdout + result.stderr;
      expect(output).not.toContain("Missing folder.meta.yaml");
    });
  });

  test.skipIf(!process.env["EE_LICENSE_KEY"])("non-admin user gets error and exit code 1", async () => {
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const nonAdminToken = await createNonAdminUser(backend);

      const uniqueId = Date.now();
      const folderName = `nometanonadmin${uniqueId}`;

      await writeFile(join(tempDir, "wmill.yaml"), wmillYaml(), "utf-8");

      // Create a script inside a folder WITHOUT folder.meta.yaml
      // First create the folder on remote so the non-admin has somewhere to push
      await backend.apiRequest(
        `/api/w/${backend.workspace}/folders/create`,
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

      const result = await backend.runCLICommand(
        ["sync", "push", "--yes", "--includes", `f/${folderName}/**`],
        tempDir,
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
    await withTestBackend(async (backend, tempDir) => {
      await setupWorkspaceProfile(backend);

      const uniqueId = Date.now();
      const folderName = `delfolder${uniqueId}`;

      // Create folder and script on remote via API
      await backend.apiRequest!(
        `/api/w/${backend.workspace}/folders/create`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ name: folderName }),
        }
      );

      await writeFile(join(tempDir, "wmill.yaml"), wmillYaml(), "utf-8");

      // Pull to get remote state, then delete the folder locally
      await backend.runCLICommand(["sync", "pull", "--yes"], tempDir);

      // Remove the folder locally to trigger a "deleted" change
      await rm(join(tempDir, "f", folderName), { recursive: true, force: true });

      const result = await backend.runCLICommand(
        ["sync", "push", "--yes", "--includes", `f/${folderName}/**`],
        tempDir
      );

      // Should not warn about missing meta for deleted items
      const output = result.stdout + result.stderr;
      expect(output).not.toContain("Missing folder.meta.yaml");
    });
  });
});
