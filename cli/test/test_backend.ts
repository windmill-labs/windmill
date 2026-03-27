/**
 * Unified Test Backend Interface
 *
 * Provides a common interface for both Docker-based and Cargo-based backends.
 * Use environment variable TEST_BACKEND to switch:
 *   - TEST_BACKEND=cargo (default) - Uses pre-built binary + local postgres
 *   - TEST_BACKEND=docker - Uses docker-compose (legacy)
 *
 * Prerequisites for cargo backend:
 *   - PostgreSQL running locally (default: postgres://postgres:changeme@localhost:5432)
 *   - Backend built: cd backend && cargo build --release (or debug)
 *
 * Usage:
 *   import { withTestBackend, cleanupTestBackend } from "./test_backend.ts";
 *
 *   test("my test", async () => {
 *     await withTestBackend(async (backend, tempDir) => {
 *       const result = await backend.runCLICommand(["sync", "pull"], tempDir);
 *       // ...
 *     });
 *   });
 *
 * CROSS-LINKS - Related test helper locations (keep in sync when adding new helpers):
 * @see test_fixtures.ts - Local file fixtures (createLocalScript, createLocalFlow, etc.)
 * @see sync_pull_push.test.ts - Local fixtures + createRemoteScript (API-based)
 *
 * This file contains: API-based creation helpers (createTestApp, createTestResource,
 *   createAppWithInlineScript, createFlowWithInlineScript, etc.)
 * If you add new helpers, update cross-links in the files above.
 */

import { CargoBackend, CargoBackendConfig, cleanupStaleTestResources } from "./cargo_backend.ts";
import { ContainerizedBackend, ContainerConfig } from "./containerized_backend.ts";
import { mkdtemp, rm } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";

/**
 * Common interface for test backends
 */
export interface TestBackend {
  readonly baseUrl: string;
  readonly workspace: string;
  readonly testConfigDir: string;
  readonly token?: string;

  start(): Promise<void>;
  stop(): Promise<void>;
  reset(): Promise<void>;

  createCLICommand(args: string[], workingDir: string, opts?: { workspace?: string; token?: string }): any;
  runCLICommand(args: string[], workingDir: string, opts?: { workspace?: string; token?: string }): Promise<{
    stdout: string;
    stderr: string;
    code: number;
  }>;

  // Optional methods that may not be on all backends
  apiRequest?(path: string, options?: RequestInit): Promise<Response>;
  seedTestData?(): Promise<void>;
  getWorkspaceSettings?(): Promise<any>;
  updateGitSyncConfig?(config: any): Promise<void>;
  createAdditionalGitRepo?(repoPath: string, description: string): Promise<void>;
  listAllScripts?(): Promise<any[]>;
  listAllApps?(): Promise<any[]>;
  listAllResources?(): Promise<any[]>;
  listAllVariables?(): Promise<any[]>;

  // Methods for creating apps and flows with custom inline scripts
  createAppWithInlineScript?(path: string, inlineScriptContent: string, language?: string): Promise<void>;
  createFlowWithInlineScript?(path: string, inlineScriptContent: string, language?: string): Promise<void>;
}

/**
 * Adapter to make CargoBackend implement TestBackend
 */
class CargoBackendAdapter implements TestBackend {
  private backend: CargoBackend;

  constructor(config?: Partial<CargoBackendConfig>) {
    this.backend = new CargoBackend(config);
  }

  get baseUrl(): string {
    return this.backend.baseUrl;
  }

  get workspace(): string {
    return this.backend.workspace;
  }

  get testConfigDir(): string {
    return this.backend.testConfigDir;
  }

  get token(): string {
    return this.backend.authToken;
  }

  async start(): Promise<void> {
    await this.backend.start();
  }

  async stop(): Promise<void> {
    await this.backend.stop();
  }

  async reset(): Promise<void> {
    await this.backend.reset();
  }

  createCLICommand(args: string[], workingDir: string, opts?: { workspace?: string; token?: string }): any {
    return this.backend.createCLICommand(args, workingDir, opts);
  }

  async runCLICommand(args: string[], workingDir: string, opts?: { workspace?: string; token?: string }) {
    return this.backend.runCLICommand(args, workingDir, opts);
  }

  async apiRequest(path: string, options?: RequestInit): Promise<Response> {
    return this.backend.apiRequest(path, options);
  }

  /** Seeds test data via API calls. See file header for cross-links to related helpers. */
  async seedTestData(): Promise<void> {
    // Create test folder first
    await this.createTestFolder("test");

    // Create test resources and variables
    await this.createTestResource("f/test/my_resource", "Test resource description");
    await this.createTestVariable("f/test/my_variable", "Test variable value");

    // Create test group
    await this.createTestGroup("test_group");

    // Create a test app
    await this.createTestApp("f/test/test_dashboard");
  }

  /** See file header for cross-links to related helpers. */
  private async createTestApp(path: string): Promise<void> {
    const response = await this.backend.apiRequest(`/api/w/${this.workspace}/apps/create`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        path,
        value: {
          type: "app",
          grid: [],
          hiddenInlineScripts: [],
          css: {},
          norefreshbar: false,
        },
        summary: "Test app",
        policy: {
          on_behalf_of: null,
          on_behalf_of_email: null,
          triggerables: {},
          execution_mode: "viewer",
        },
      }),
    });
    if (!response.ok) {
      const error = await response.text();
      if (!error.includes("already exists")) {
        console.warn(`Warning: Failed to create app ${path}: ${error}`);
      }
    } else {
      await response.text();
    }
  }

  /** See file header for cross-links to related helpers. */
  private async createTestFolder(name: string): Promise<void> {
    const response = await this.backend.apiRequest(`/api/w/${this.workspace}/folders/create`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ name }),
    });
    if (!response.ok) {
      const error = await response.text();
      if (!error.includes("already exists")) {
        console.warn(`Warning: Failed to create folder ${name}: ${error}`);
      }
    } else {
      await response.text();
    }
  }

  /** See file header for cross-links to related helpers. */
  private async createTestGroup(name: string): Promise<void> {
    const response = await this.backend.apiRequest(`/api/w/${this.workspace}/groups/create`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ name, summary: `Test group ${name}` }),
    });
    if (!response.ok) {
      const error = await response.text();
      if (!error.includes("already exists")) {
        console.warn(`Warning: Failed to create group ${name}: ${error}`);
      }
    } else {
      await response.text();
    }
  }


  /** See file header for cross-links to related helpers. */
  private async createTestResource(path: string, description: string): Promise<void> {
    // First ensure the folder exists
    const folderPath = path.split("/").slice(0, 2).join("/"); // e.g., "f/test"
    const folderName = folderPath.replace("f/", "");

    try {
      const folderResponse = await this.backend.apiRequest(`/api/w/${this.workspace}/folders/create`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name: folderName }),
      });
      await folderResponse.text(); // Consume response body
    } catch {
      // Folder may already exist
    }

    const response = await this.backend.apiRequest(
      `/api/w/${this.workspace}/resources/create`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          path,
          description,
          resource_type: "any",
          value: { test: "value" },
        }),
      }
    );
    if (!response.ok) {
      const error = await response.text();
      if (!error.includes("already exists")) {
        console.warn(`Warning: Failed to create resource ${path}: ${error}`);
      }
    } else {
      await response.text();
    }
  }

  /** See file header for cross-links to related helpers. */
  private async createTestVariable(path: string, value: string): Promise<void> {
    const response = await this.backend.apiRequest(
      `/api/w/${this.workspace}/variables/create`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          path,
          value,
          is_secret: false,
          description: "Test variable",
        }),
      }
    );
    if (!response.ok) {
      const error = await response.text();
      if (!error.includes("already exists")) {
        console.warn(`Warning: Failed to create variable ${path}: ${error}`);
      }
    } else {
      await response.text();
    }
  }

  async getWorkspaceSettings(): Promise<any> {
    const response = await this.backend.apiRequest(`/api/w/${this.workspace}/workspaces/get_settings`);
    if (!response.ok) {
      throw new Error(`Failed to get workspace settings: ${response.status}`);
    }
    return response.json();
  }

  async updateGitSyncConfig(config: any): Promise<void> {
    const response = await this.backend.apiRequest(
      `/api/w/${this.workspace}/workspaces/edit_git_sync_config`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(config),
      }
    );
    if (!response.ok) {
      throw new Error(`Failed to update git sync config: ${response.status}`);
    }
    await response.text();
  }

  async createAdditionalGitRepo(repoPath: string, description: string): Promise<void> {
    const gitRepo = {
      path: repoPath,
      description,
      resource_type: "git_repository",
      value: {
        url: "https://github.com/windmill-labs/windmill-test.git",
        branch: "main",
        token: "",
      },
    };

    const response = await this.backend.apiRequest(
      `/api/w/${this.workspace}/resources/create`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(gitRepo),
      }
    );

    if (!response.ok) {
      const error = await response.text();
      if (!error.includes("already exists")) {
        console.warn(`Failed to create git repo ${repoPath}: ${error}`);
      }
    } else {
      await response.text();
    }
  }

  async listAllScripts(): Promise<any[]> {
    const response = await this.backend.apiRequest(`/api/w/${this.workspace}/scripts/list`);
    if (!response.ok) return [];
    return response.json();
  }

  async listAllApps(): Promise<any[]> {
    const response = await this.backend.apiRequest(`/api/w/${this.workspace}/apps/list`);
    if (!response.ok) return [];
    return response.json();
  }

  async listAllResources(): Promise<any[]> {
    const response = await this.backend.apiRequest(`/api/w/${this.workspace}/resources/list`);
    if (!response.ok) return [];
    return response.json();
  }

  async listAllVariables(): Promise<any[]> {
    const response = await this.backend.apiRequest(`/api/w/${this.workspace}/variables/list`);
    if (!response.ok) return [];
    return response.json();
  }

  async createAppWithInlineScript(path: string, inlineScriptContent: string, language: string = "bun"): Promise<void> {
    const response = await this.backend.apiRequest(`/api/w/${this.workspace}/apps/create`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        path,
        value: {
          type: "app",
          grid: [
            {
              id: "button1",
              data: {
                type: "buttoncomponent",
                componentInput: {
                  type: "runnable",
                  runnable: {
                    type: "runnableByName",
                    inlineScript: {
                      content: inlineScriptContent,
                      language,
                    },
                  },
                },
              },
            },
          ],
          hiddenInlineScripts: [],
          css: {},
          norefreshbar: false,
        },
        summary: "Test app with inline script",
        policy: {
          on_behalf_of: null,
          on_behalf_of_email: null,
          triggerables: {},
          execution_mode: "viewer",
        },
      }),
    });
    if (!response.ok) {
      const error = await response.text();
      throw new Error(`Failed to create app ${path}: ${error}`);
    }
    await response.text();
  }

  async createFlowWithInlineScript(path: string, inlineScriptContent: string, language: string = "bun"): Promise<void> {
    const response = await this.backend.apiRequest(`/api/w/${this.workspace}/flows/create`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        path,
        summary: "Test flow with inline script",
        description: `Flow at ${path}`,
        value: {
          modules: [
            {
              id: "a",
              value: {
                type: "rawscript",
                content: inlineScriptContent,
                language,
                input_transforms: {},
              },
            },
          ],
        },
        schema: {
          $schema: "https://json-schema.org/draft/2020-12/schema",
          type: "object",
          properties: {},
          required: [],
        },
      }),
    });
    if (!response.ok) {
      const error = await response.text();
      throw new Error(`Failed to create flow ${path}: ${error}`);
    }
    await response.text();
  }
}

/**
 * Adapter to make ContainerizedBackend implement TestBackend
 */
class ContainerizedBackendAdapter implements TestBackend {
  private backend: ContainerizedBackend;

  constructor(config?: Partial<ContainerConfig>) {
    this.backend = new ContainerizedBackend(config);
  }

  get baseUrl(): string {
    return this.backend.baseUrl;
  }

  get workspace(): string {
    return this.backend.workspace;
  }

  get testConfigDir(): string {
    return this.backend.testConfigDir;
  }

  get token(): string {
    return this.backend.token;
  }

  async start(): Promise<void> {
    await this.backend.start();
  }

  async stop(): Promise<void> {
    await this.backend.stop();
  }

  async reset(): Promise<void> {
    await this.backend.reset();
  }

  createCLICommand(args: string[], workingDir: string, opts?: { workspace?: string; token?: string }): any {
    return this.backend.createCLICommand(args, workingDir, opts);
  }

  async runCLICommand(args: string[], workingDir: string, opts?: { workspace?: string; token?: string }) {
    return this.backend.runCLICommand(args, workingDir, opts);
  }

  async seedTestData(): Promise<void> {
    await this.backend.seedTestData();
  }

  async getWorkspaceSettings(): Promise<any> {
    return this.backend.getWorkspaceSettings();
  }

  async updateGitSyncConfig(config: any): Promise<void> {
    await this.backend.updateGitSyncConfig(config);
  }

  async createAdditionalGitRepo(repoPath: string, description: string): Promise<void> {
    await this.backend.createAdditionalGitRepo(repoPath, description);
  }

  async listAllScripts(): Promise<any[]> {
    return this.backend.listAllScripts();
  }

  async listAllApps(): Promise<any[]> {
    return this.backend.listAllApps();
  }

  async listAllResources(): Promise<any[]> {
    return this.backend.listAllResources();
  }

  async listAllVariables(): Promise<any[]> {
    return this.backend.listAllVariables();
  }
}

// Global backend instance
let globalBackend: TestBackend | null = null;

/**
 * Get the backend type from environment
 */
function getBackendType(): "cargo" | "docker" {
  const envType = process.env["TEST_BACKEND"]?.toLowerCase();
  if (envType === "docker") {
    return "docker";
  }
  return "cargo"; // Default to cargo
}

/**
 * Create a new backend instance based on configuration
 */
export function createTestBackend(type?: "cargo" | "docker"): TestBackend {
  const backendType = type || getBackendType();

  if (backendType === "docker") {
    console.log("📦 Using Docker-based test backend");
    return new ContainerizedBackendAdapter();
  } else {
    console.log("🦀 Using Cargo-based test backend");
    return new CargoBackendAdapter({
      verbose: process.env["VERBOSE"] === "1",
    });
  }
}

/**
 * Get or create global backend instance
 */
export async function getTestBackend(): Promise<TestBackend> {
  if (!globalBackend) {
    // Clean up stale databases and processes from previous crashed test runs
    await cleanupStaleTestResources();
    globalBackend = createTestBackend();
    registerCleanup();
    await globalBackend.start();
  }
  return globalBackend;
}

/**
 * Convenience function for tests - runs test with backend
 */
export async function withTestBackend<T>(
  testFn: (backend: TestBackend, tempDir: string) => Promise<T>
): Promise<T> {
  const backend = await getTestBackend();
  const tempDir = await mkdtemp(join(tmpdir(), "windmill_cli_test_"));

  try {
    await backend.reset();
    if (backend.seedTestData) {
      await backend.seedTestData();
    }
    return await testFn(backend, tempDir);
  } finally {
    await rm(tempDir, { recursive: true });
  }
}

/**
 * Cleanup function for test suites
 */
export async function cleanupTestBackend(): Promise<void> {
  if (globalBackend) {
    await globalBackend.stop();
    globalBackend = null;
  }
}

// Auto-cleanup on process exit
let cleanupRegistered = false;
function registerCleanup() {
  if (cleanupRegistered) return;
  cleanupRegistered = true;
  process.on("exit", () => {
    if (globalBackend) {
      // Synchronous kill — can't await in exit handler.
      // Kill the direct child (cargo); any orphaned windmill child processes
      // will be cleaned up by cleanupStaleTestResources() on next startup.
      const pid = (globalBackend as any).backend?.process?.pid;
      if (pid) {
        try {
          process.kill(pid, "SIGKILL");
        } catch {
          // Best effort — process may already be dead
        }
      }
    }
  });
  // Handle graceful shutdown
  for (const signal of ["SIGINT", "SIGTERM"] as const) {
    process.on(signal, async () => {
      await cleanupTestBackend();
      process.exit(0);
    });
  }
}

/**
 * Create a non-admin user, add them to the workspace, and return their token.
 */
export async function createNonAdminUser(
  backend: TestBackend,
  workspaceId: string = backend.workspace
): Promise<string> {
  if (!backend.apiRequest) {
    throw new Error("Backend does not support apiRequest");
  }

  const email = `nonadmin_${Date.now()}@test.dev`;
  const password = "testpass123";

  // Create user globally (as admin)
  const createResp = await backend.apiRequest("/api/users/create", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      email,
      password,
      super_admin: false,
      name: "Non-Admin Test User",
    }),
  });
  if (!createResp.ok) {
    throw new Error(`Failed to create user: ${await createResp.text()}`);
  }
  await createResp.text();

  // Add user to workspace as non-admin
  const addResp = await backend.apiRequest(
    `/api/w/${workspaceId}/workspaces/add_user`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        email,
        is_admin: false,
        operator: false,
      }),
    }
  );
  if (!addResp.ok) {
    throw new Error(`Failed to add user to workspace: ${await addResp.text()}`);
  }
  await addResp.text();

  // Login as the non-admin user to get a token
  const loginResp = await fetch(`${backend.baseUrl}/api/auth/login`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ email, password }),
  });
  if (!loginResp.ok) {
    throw new Error(`Failed to login as non-admin: ${await loginResp.text()}`);
  }
  return await loginResp.text();
}

/**
 * Create workspace dependencies via the API (e.g. a shared package.json for bun scripts).
 */
export async function createRemoteWorkspaceDeps(
  backend: TestBackend,
  language: string,
  content: string,
  name?: string,
): Promise<void> {
  if (!backend.apiRequest) {
    throw new Error("Backend does not support apiRequest");
  }

  const resp = await backend.apiRequest(
    `/api/w/${backend.workspace}/workspace_dependencies/create`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        workspace_id: backend.workspace,
        language,
        content,
        ...(name ? { name } : {}),
      }),
    }
  );
  if (!resp.ok) {
    throw new Error(`Failed to create workspace deps (${resp.status}): ${await resp.text()}`);
  }
  await resp.text();
}

// Re-export for convenience
export type { CargoBackendConfig } from "./cargo_backend.ts";
export type { ContainerConfig } from "./containerized_backend.ts";
