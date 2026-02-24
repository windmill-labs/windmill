/**
 * Cargo-based Backend Test Utilities
 * Runs Windmill backend directly via `cargo run` for CLI testing
 *
 * Prerequisites:
 * - PostgreSQL server running (default: localhost:5432)
 * - Rust toolchain installed
 * - Backend code compiled or ready to compile
 *
 * Usage:
 *   DATABASE_URL=postgres://postgres:changeme@localhost:5432 bun test test/my_test.ts
 */

import { resolve, dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { statSync } from "node:fs";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { createServer } from "node:net";
import { Subprocess } from "bun";

export interface CargoBackendConfig {
  /** PostgreSQL connection string (without database name) */
  postgresUrl?: string;
  /** Port for the backend server (0 = auto-select) */
  port?: number;
  /** Path to the backend directory */
  backendDir?: string;
  /** Path to pre-built windmill binary (optional, uses cargo run if not set) */
  binaryPath?: string;
  /** Cargo features to enable (default: ["zip"]) */
  features?: string[];
  /** Use release build (default: false) */
  release?: boolean;
  /** Workspace ID for tests */
  workspace?: string;
  /** Admin username */
  username?: string;
  /** Admin password */
  password?: string;
  /** Timeout for backend startup (ms) */
  timeout?: number;
  /** Test config directory */
  testConfigDir?: string;
  /** Enable verbose output */
  verbose?: boolean;
}

export class CargoBackend {
  private config: Required<CargoBackendConfig>;
  private process: Subprocess | null = null;
  private dbName: string;
  private isRunning = false;
  private actualPort: number;
  private token = "";

  constructor(config: Partial<CargoBackendConfig> = {}) {
    // Generate unique database name for this test run
    this.dbName = `windmill_test_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
    this.actualPort = config.port || 0;

    const backendDir = config.backendDir || this.findBackendDir();

    // Determine default features based on environment
    // CI mode: minimal features (zip only)
    // Local mode with license key: full features (zip, private, enterprise, license)
    // Local mode without license key: zip only (EE features reject API calls without valid license)
    const isCI = process.env["CI_MINIMAL_FEATURES"] === "true";
    const hasLicenseKey = !!process.env["EE_LICENSE_KEY"];
    const defaultFeatures = isCI ? ["zip"] : (hasLicenseKey ? ["zip", "private", "enterprise", "license"] : ["zip"]);

    // Parse additional features from environment variable
    const envFeatures = process.env["TEST_FEATURES"]?.split(",").filter(f => f.trim()) || [];
    const allFeatures = [...new Set([...defaultFeatures, ...envFeatures, ...(config.features || [])])];

    this.config = {
      postgresUrl: config.postgresUrl || process.env["DATABASE_URL"] || "postgres://postgres:changeme@localhost:5432",
      port: config.port || 0,
      backendDir,
      binaryPath: config.binaryPath || process.env["WINDMILL_BINARY"] || "",
      features: allFeatures,
      release: config.release ?? false,
      workspace: config.workspace || "test",
      username: config.username || "admin@windmill.dev",
      password: config.password || "changeme",
      timeout: config.timeout || 120000,
      testConfigDir: config.testConfigDir || "",
      verbose: config.verbose || false,
    };
  }

  private findBackendDir(): string {
    // Try to find backend directory relative to CLI
    const cliTestDir = dirname(fileURLToPath(import.meta.url));
    // Use resolve() for proper cross-platform path resolution
    const candidates = [
      resolve(cliTestDir, "..", "..", "backend"),
      resolve(cliTestDir, "..", "..", "..", "backend"),
      resolve(".", "backend"),
      resolve("..", "backend"),
    ];

    for (const candidate of candidates) {
      try {
        const cargoPath = resolve(candidate, "Cargo.toml");
        const stat = statSync(cargoPath);
        if (stat.isFile()) {
          return candidate;
        }
      } catch {
        // Continue searching
      }
    }

    throw new Error("Could not find backend directory. Set backendDir in config.");
  }

  get baseUrl(): string {
    return `http://localhost:${this.actualPort}`;
  }

  get workspace(): string {
    return this.config.workspace;
  }

  get testConfigDir(): string {
    return this.config.testConfigDir;
  }

  /**
   * Start the backend server
   */
  async start(): Promise<void> {
    if (this.isRunning) {
      return;
    }

    console.log("Starting Cargo-based Windmill backend...");

    // Create test config directory
    if (!this.config.testConfigDir) {
      this.config.testConfigDir = await mkdtemp(join(tmpdir(), "wmill_test_config_"));
      console.log(`Created test config directory: ${this.config.testConfigDir}`);
    }

    // Find a free port if not specified
    if (this.actualPort === 0) {
      this.actualPort = await this.findFreePort();
    }
    console.log(`Using port: ${this.actualPort}`);

    // Create the test database
    await this.createDatabase();

    // Start the backend
    await this.startBackendProcess();

    // Wait for API to be ready
    await this.waitForAPI();

    // Initialize test data and authenticate
    await this.initializeAndAuthenticate();

    this.isRunning = true;
    console.log("Cargo backend is ready!");
    console.log(`   Server: ${this.baseUrl}`);
    console.log(`   Database: ${this.dbName}`);
    console.log(`   Workspace: ${this.config.workspace}`);
  }

  /**
   * Stop the backend server and cleanup
   */
  async stop(): Promise<void> {
    if (!this.isRunning) {
      return;
    }

    console.log("Stopping Cargo backend...");

    // Kill the backend process
    if (this.process) {
      try {
        this.process.kill();
        // Wait a bit for graceful shutdown
        await Promise.race([
          this.process.exited,
          new Promise(resolve => setTimeout(resolve, 5000)),
        ]);
      } catch {
        // Process may already be dead
      }
      this.process = null;
    }

    // Drop the test database
    await this.dropDatabase();

    // Cleanup test config directory
    if (this.config.testConfigDir?.includes("wmill_test_config_")) {
      try {
        await rm(this.config.testConfigDir, { recursive: true, force: true });
        console.log(`Cleaned up test config directory`);
      } catch {
        // Ignore cleanup errors
      }
    }

    this.isRunning = false;
    console.log("Backend stopped");
  }

  /**
   * Find a free port
   */
  private async findFreePort(): Promise<number> {
    return new Promise((resolve, reject) => {
      const server = createServer();
      server.listen(0, () => {
        const port = (server.address() as any).port;
        server.close(() => resolve(port));
      });
      server.on('error', reject);
    });
  }

  /**
   * Parse PostgreSQL URL and return base URL (without database name)
   * Handles both formats:
   * - postgres://user:pass@host:port/database
   * - postgres://user:pass@host:port (no database)
   */
  private getBasePostgresUrl(): string {
    const url = new URL(this.config.postgresUrl);
    // Remove any existing database path and query params (e.g. ?sslmode=disable)
    url.pathname = "";
    url.search = "";
    return url.toString().replace(/\/$/, ""); // Remove trailing slash
  }

  /**
   * Create the test database
   */
  private async createDatabase(): Promise<void> {
    console.log(`Creating test database: ${this.dbName}`);

    const baseUrl = this.getBasePostgresUrl();

    const proc = Bun.spawn(["psql", `${baseUrl}/postgres`, "-c", `CREATE DATABASE "${this.dbName}";`], {
      stdout: "pipe",
      stderr: "pipe",
    });

    const [stdout, stderr] = await Promise.all([
      new Response(proc.stdout).text(),
      new Response(proc.stderr).text(),
    ]);
    const exitCode = await proc.exited;

    if (exitCode !== 0) {
      throw new Error(`Failed to create database: ${stderr}`);
    }

    console.log("Test database created");
  }

  /**
   * Drop the test database
   */
  private async dropDatabase(): Promise<void> {
    console.log(`Dropping test database: ${this.dbName}`);

    const baseUrl = this.getBasePostgresUrl();

    // Terminate existing connections
    const terminateProc = Bun.spawn(["psql", `${baseUrl}/postgres`, "-c",
      `SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '${this.dbName}' AND pid <> pg_backend_pid();`], {
      stdout: "pipe",
      stderr: "pipe",
    });
    await Promise.all([
      new Response(terminateProc.stdout).text(),
      new Response(terminateProc.stderr).text(),
    ]);
    await terminateProc.exited;

    // Drop the database
    const dropProc = Bun.spawn(["psql", `${baseUrl}/postgres`, "-c",
      `DROP DATABASE IF EXISTS "${this.dbName}";`], {
      stdout: "pipe",
      stderr: "pipe",
    });

    const [, stderr] = await Promise.all([
      new Response(dropProc.stdout).text(),
      new Response(dropProc.stderr).text(),
    ]);
    const exitCode = await dropProc.exited;

    if (exitCode !== 0) {
      console.warn(`Warning: Failed to drop database: ${stderr}`);
    } else {
      console.log("Test database dropped");
    }
  }

  /**
   * Start the backend process using cargo run
   */
  private stderrChunks: Uint8Array[] = [];
  private stdoutChunks: Uint8Array[] = [];

  private async startBackendProcess(): Promise<void> {
    const baseUrl = this.getBasePostgresUrl();
    const databaseUrl = `${baseUrl}/${this.dbName}?sslmode=disable`;

    const env: Record<string, string> = {
      ...process.env as Record<string, string>,
      DATABASE_URL: databaseUrl,
      PORT: String(this.actualPort),
      MODE: "standalone", // Run server + worker in one process
      RUST_LOG: "info",
      DISABLE_TELEMETRY: "true",
      METRICS_ENABLED: "false",
      NUM_WORKERS: "1",
      SLEEP_QUEUE: "50",
      // Required for sqlx compile-time checks when using cargo run
      SQLX_OFFLINE: "true",
      // Disable embedding to speed up startup
      DISABLE_EMBEDDING: "true",
      // Create default admin user
      CREATE_SUPERADMIN_IF_NOT_EXISTS: "1",
      SUPERADMIN_EMAIL: this.config.username,
      SUPERADMIN_PASSWORD: this.config.password,
    };

    // On Windows, ensure BUN_PATH and NODE_BIN_PATH are set for the worker.
    // The Rust defaults (/usr/bin/bun, /usr/bin/node) don't exist on Windows.
    if (process.platform === "win32") {
      env.BUN_PATH = env.BUN_PATH || Bun.which("bun") || process.execPath;
      env.NODE_BIN_PATH = env.NODE_BIN_PATH || Bun.which("node") || "node";
    }

    // Add license key if available
    const licenseKey = process.env["EE_LICENSE_KEY"];
    if (licenseKey) {
      env.LICENSE_KEY = licenseKey;
    }

    if (this.config.binaryPath) {
      // Use pre-built binary if explicitly specified
      console.log(`Starting backend using binary: ${this.config.binaryPath}`);
      console.log(`   DATABASE_URL: ${databaseUrl}`);

      this.process = Bun.spawn([this.config.binaryPath], {
        env,
        stdout: "pipe",
        stderr: "pipe",
      });
    } else {
      // Use cargo run with features
      const cargoArgs = ["run"];
      if (this.config.release) {
        cargoArgs.push("--release");
      }
      if (this.config.features.length > 0) {
        cargoArgs.push("--features", this.config.features.join(","));
      }

      console.log(`Starting backend via: cargo ${cargoArgs.join(" ")}`);
      console.log(`   DATABASE_URL: ${databaseUrl}`);
      console.log(`   Backend dir: ${this.config.backendDir}`);

      this.process = Bun.spawn(["cargo", ...cargoArgs], {
        cwd: this.config.backendDir,
        env,
        stdout: "pipe",
        stderr: "pipe",
      });
    }

    this.stderrChunks = [];
    this.stdoutChunks = [];

    // Capture output in background
    this.captureProcessOutput();

    console.log(`Backend process started (PID: ${this.process.pid})`);
  }

  /**
   * Capture process output for debugging
   */
  private captureProcessOutput(): void {
    if (!this.process) return;

    const stdout = this.process.stdout;
    const stderr = this.process.stderr;

    if (stdout) {
      (async () => {
        const reader = stdout.getReader();
        try {
          while (true) {
            const { done, value } = await reader.read();
            if (done) break;
            if (value) {
              this.stdoutChunks.push(value);
              if (this.config.verbose) {
                process.stdout.write(value);
              }
            }
          }
        } catch {
          // Process may have exited
        }
      })();
    }

    if (stderr) {
      (async () => {
        const reader = stderr.getReader();
        try {
          while (true) {
            const { done, value } = await reader.read();
            if (done) break;
            if (value) {
              this.stderrChunks.push(value);
              if (this.config.verbose) {
                process.stderr.write(value);
              }
            }
          }
        } catch {
          // Process may have exited
        }
      })();
    }
  }

  /**
   * Get captured stderr output
   */
  private getStderr(): string {
    const totalLength = this.stderrChunks.reduce((sum, chunk) => sum + chunk.length, 0);
    const combined = new Uint8Array(totalLength);
    let offset = 0;
    for (const chunk of this.stderrChunks) {
      combined.set(chunk, offset);
      offset += chunk.length;
    }
    return new TextDecoder().decode(combined);
  }

  /**
   * Get captured stdout output
   */
  private getStdout(): string {
    const totalLength = this.stdoutChunks.reduce((sum, chunk) => sum + chunk.length, 0);
    const combined = new Uint8Array(totalLength);
    let offset = 0;
    for (const chunk of this.stdoutChunks) {
      combined.set(chunk, offset);
      offset += chunk.length;
    }
    return new TextDecoder().decode(combined);
  }

  /**
   * Wait for the API to be responsive
   */
  private async waitForAPI(): Promise<void> {
    console.log("Waiting for API to be responsive (this may take a few minutes if compiling)...");

    // Allow up to 10 minutes for cargo build + startup
    const maxAttempts = 300; // 10 minutes with 2-second intervals
    let attempts = 0;
    let lastProgressLog = Date.now();

    while (attempts < maxAttempts) {
      try {
        const response = await fetch(`${this.baseUrl}/api/version`, {
          signal: AbortSignal.timeout(5000),
        });

        if (response.ok) {
          const version = await response.text();
          console.log(`API ready (version: ${version.trim()})`);
          return;
        }
        await response.text(); // Consume response
      } catch {
        // Continue trying
      }

      // Check if process died
      if (this.process) {
        try {
          const status = await Promise.race([
            this.process.exited,
            new Promise<null>(resolve => setTimeout(() => resolve(null), 100)),
          ]);
          if (status !== null) {
            // Wait a bit for output to be captured
            await new Promise(resolve => setTimeout(resolve, 500));
            const stderr = this.getStderr();
            const stdout = this.getStdout();
            console.error("\nBackend process crashed!");
            if (stdout) {
              console.error("=== STDOUT ===\n" + stdout.slice(-2000));
            }
            if (stderr) {
              console.error("=== STDERR ===\n" + stderr.slice(-2000));
            }
            throw new Error(`Backend process exited with code ${status}`);
          }
        } catch (e) {
          if (e instanceof Error && e.message.includes("exited")) {
            throw e;
          }
        }
      }

      attempts++;

      // Log progress every 30 seconds
      if (Date.now() - lastProgressLog > 30000) {
        const elapsedMin = Math.floor((attempts * 2) / 60);
        const elapsedSec = (attempts * 2) % 60;
        console.log(`   Still waiting... (${elapsedMin}m ${elapsedSec}s elapsed, compiling...)`);
        lastProgressLog = Date.now();
      }

      await new Promise(resolve => setTimeout(resolve, 2000));
    }

    throw new Error("API failed to respond within timeout (10 minutes)");
  }

  /**
   * Initialize test data and authenticate
   */
  private async initializeAndAuthenticate(): Promise<void> {
    console.log("Initializing test workspace...");

    // Create test workspace via API
    await this.createWorkspace();

    // Login to get token
    await this.authenticate();

    console.log("Test workspace initialized");
  }

  /**
   * Create the test workspace
   */
  private async createWorkspace(): Promise<void> {
    // First login as superadmin to create workspace
    const loginResponse = await fetch(`${this.baseUrl}/api/auth/login`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        email: this.config.username,
        password: this.config.password,
      }),
    });

    if (!loginResponse.ok) {
      throw new Error(`Login failed: ${loginResponse.status}`);
    }

    const tempToken = await loginResponse.text();

    // Create workspace
    const createWsResponse = await fetch(`${this.baseUrl}/api/workspaces/create`, {
      method: "POST",
      headers: {
        "Authorization": `Bearer ${tempToken}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        id: this.config.workspace,
        name: "Test Workspace",
      }),
    });

    if (!createWsResponse.ok) {
      const error = await createWsResponse.text();
      // Workspace may already exist
      if (!error.includes("already exists") && !error.includes("duplicate")) {
        console.warn(`Warning: Failed to create workspace: ${error}`);
      }
    } else {
      await createWsResponse.text();
      console.log(`  Created workspace: ${this.config.workspace}`);
    }
  }

  /**
   * Authenticate and get token
   */
  private async authenticate(): Promise<void> {
    console.log("Authenticating...");

    const loginResponse = await fetch(`${this.baseUrl}/api/auth/login`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        email: this.config.username,
        password: this.config.password,
      }),
    });

    if (!loginResponse.ok) {
      throw new Error(`Authentication failed: ${loginResponse.status}`);
    }

    this.token = await loginResponse.text();
    console.log("Authentication successful");
  }

  /**
   * Get authentication token
   */
  get authToken(): string {
    return this.token;
  }

  /**
   * Create CLI command with proper authentication
   */
  createCLICommand(args: string[], workingDir: string, opts?: { workspace?: string; token?: string }): { command: string, args: string[], cwd: string, env: Record<string, string> } {
    const workspace = opts?.workspace || this.config.workspace;
    const cliDir = join(dirname(fileURLToPath(import.meta.url)), "..");
    const fullArgs = [
      "--base-url", this.baseUrl,
      "--workspace", workspace,
      "--token", opts?.token || this.token,
      "--config-dir", this.config.testConfigDir,
      ...args,
    ];

    const useNode = process.env["TEST_CLI_RUNTIME"] === "node";
    const runtime = useNode ? "node" : "bun";
    const entrypoint = useNode
      ? join(cliDir, "npm", "esm", "main.js")
      : join(cliDir, "src", "main.ts");
    const runtimeArgs = useNode ? [entrypoint] : ["run", entrypoint];

    console.log("CLI Command:", [runtime, ...runtimeArgs, ...fullArgs].join(" "));

    return {
      command: runtime,
      args: [...runtimeArgs, ...fullArgs],
      cwd: workingDir,
      env: { ...process.env as Record<string, string> },
    };
  }

  /**
   * Run CLI command and return result
   */
  async runCLICommand(args: string[], workingDir: string, opts?: { workspace?: string; token?: string }): Promise<{
    stdout: string;
    stderr: string;
    code: number;
  }> {
    const cmd = this.createCLICommand(args, workingDir, opts);
    const proc = Bun.spawn([cmd.command, ...cmd.args], {
      cwd: cmd.cwd,
      env: cmd.env,
      stdout: "pipe",
      stderr: "pipe",
    });

    const [stdout, stderr] = await Promise.all([
      new Response(proc.stdout).text(),
      new Response(proc.stderr).text(),
    ]);
    const code = await proc.exited;

    return { stdout, stderr, code };
  }

  /**
   * Make authenticated API request
   */
  async apiRequest(path: string, options: RequestInit = {}): Promise<Response> {
    const url = `${this.baseUrl}${path}`;
    const headers = new Headers(options.headers);
    headers.set("Authorization", `Bearer ${this.token}`);

    return fetch(url, { ...options, headers });
  }

  /**
   * Reset workspace to clean state
   */
  async reset(): Promise<void> {
    console.log("Resetting workspace...");

    // Delete all content via API
    await Promise.all([
      this.deleteAll("scripts"),
      this.deleteAll("flows"),
      this.deleteAll("apps"),
      this.deleteAll("resources"),
      this.deleteAll("variables"),
      this.deleteAll("folders"),
    ]);

    console.log("Workspace reset complete");
  }

  private async deleteAll(resourceType: string): Promise<void> {
    try {
      const listResponse = await this.apiRequest(`/api/w/${this.config.workspace}/${resourceType}/list`);
      if (!listResponse.ok) return;

      const items = await listResponse.json();
      for (const item of items) {
        try {
          const deletePath = resourceType === "scripts"
            ? `/api/w/${this.config.workspace}/${resourceType}/delete/p/${encodeURIComponent(item.path)}`
            : `/api/w/${this.config.workspace}/${resourceType}/delete/${encodeURIComponent(item.path || item.name)}`;

          const deleteResponse = await this.apiRequest(deletePath, { method: resourceType === "scripts" ? "POST" : "DELETE" });
          await deleteResponse.text();
        } catch {
          // Ignore individual deletion failures
        }
      }
    } catch {
      // Ignore listing failures
    }
  }
}

// Global backend instance
let globalCargoBackend: CargoBackend | null = null;

/**
 * Convenience function for tests with cargo backend
 */
export async function withCargoBackend<T>(
  testFn: (backend: CargoBackend, tempDir: string) => Promise<T>,
  config?: Partial<CargoBackendConfig>
): Promise<T> {
  if (!globalCargoBackend) {
    globalCargoBackend = new CargoBackend(config);
    await globalCargoBackend.start();
  }

  const tempDir = await mkdtemp(join(tmpdir(), "windmill_cli_test_"));

  try {
    await globalCargoBackend.reset();
    return await testFn(globalCargoBackend, tempDir);
  } finally {
    await rm(tempDir, { recursive: true, force: true });
  }
}

/**
 * Cleanup function for test suites
 */
export async function cleanupCargoBackend(): Promise<void> {
  if (globalCargoBackend) {
    await globalCargoBackend.stop();
    globalCargoBackend = null;
  }
}

/**
 * Check if EE-dependent tests should be skipped
 *
 * Returns true when:
 * - CI_MINIMAL_FEATURES=true (CI mode with zip-only features)
 * - EE_LICENSE_KEY is not set (EE features reject API calls without valid license)
 *
 * Use this in test definitions:
 *   test.skipIf(shouldSkipOnCI())("my EE test", ...)
 */
export function shouldSkipOnCI(): boolean {
  return process.env["CI_MINIMAL_FEATURES"] === "true" || !process.env["EE_LICENSE_KEY"];
}
