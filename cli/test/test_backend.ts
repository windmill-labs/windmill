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
 */

import { CargoBackend, CargoBackendConfig } from "./cargo_backend.ts";
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

  createCLICommand(args: string[], workingDir: string, workspaceName?: string): any;
  runCLICommand(args: string[], workingDir: string, workspaceName?: string): Promise<{
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

  createCLICommand(args: string[], workingDir: string, workspaceName?: string): any {
    return this.backend.createCLICommand(args, workingDir, workspaceName);
  }

  async runCLICommand(args: string[], workingDir: string, workspaceName?: string) {
    return this.backend.runCLICommand(args, workingDir, workspaceName);
  }

  async apiRequest(path: string, options?: RequestInit): Promise<Response> {
    return this.backend.apiRequest(path, options);
  }

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

  createCLICommand(args: string[], workingDir: string, workspaceName?: string): any {
    return this.backend.createCLICommand(args, workingDir, workspaceName);
  }

  async runCLICommand(args: string[], workingDir: string, workspaceName?: string) {
    return this.backend.runCLICommand(args, workingDir, workspaceName);
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
      // Synchronous kill — can't await in exit handler
      try {
        (globalBackend as any).backend?.process?.kill();
      } catch {
        // Best effort
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

// Re-export for convenience
export type { CargoBackendConfig } from "./cargo_backend.ts";
export type { ContainerConfig } from "./containerized_backend.ts";
