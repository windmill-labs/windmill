#!/usr/bin/env node

import { spawn, exec } from "child_process";
import { promisify } from "util";
import * as readline from "readline";

const execAsync = promisify(exec);

export interface Config {
  dbPort: number;
  serverPort: number;
  skipBuild: boolean;
  commitHash: string;
  onCloudflaredUrl?: (url: string) => void;
  onCleanup?: () => void;
  onError?: (error: Error) => void;
}

interface SpawnedResources {
  dbContainerId: string;
  dbProcess?: any;
  backendProcess?: any;
  cloudflaredProcess?: any;
  tunnelUrl?: string;
  worktreePath?: string;
}

// No default config needed since commitHash is always required

export class EphemeralBackend {
  private config: Config;
  private resources: SpawnedResources = { dbContainerId: "" };

  getDbPort(): number {
    return this.config.dbPort;
  }
  getServerPort(): number {
    return this.config.serverPort;
  }

  constructor(config: Config) {
    this.config = config;
  }

  async spawn(): Promise<void> {
    try {
      console.log("🚀 Starting ephemeral backend...");
      console.log(`📊 Database port: ${this.config.dbPort}`);
      console.log(`🌐 Server port: ${this.config.serverPort}`);
      console.log(`📌 Commit hash: ${this.config.commitHash}`);

      await this.startCloudflared();
      await this.createWorktree();
      await this.spawnPostgres();
      await this.waitForPostgres();
      if (!this.config.skipBuild) {
        await this.buildBackend();
      } else {
        console.log("\n⏭️  Skipping backend build (using existing binary)");
      }
      await this.startBackend();

      console.log("\n✅ Ephemeral backend is ready!");
      console.log(`📍 Tunnel URL: ${this.resources.tunnelUrl}`);
      console.log("\n💡 Backend will run until cleaned up...");

      // Monitor backend process and cleanup if it crashes
      if (this.resources.backendProcess) {
        this.resources.backendProcess.on("exit", async (code: number) => {
          if (code !== 0 && code !== null) {
            console.error(`❌ Backend process exited with code ${code}`);
            const error = new Error(`Backend process exited with code ${code}`);
            this.config.onError?.(error);
            await this.cleanup();
          }
        });
      }

      // Keep the process running indefinitely
      await new Promise(() => {}); // Never resolves
    } catch (error) {
      console.error("❌ Error spawning ephemeral backend:", error);
      const err = error instanceof Error ? error : new Error(String(error));
      this.config.onError?.(err);
      await this.cleanup();
      throw error; // Re-throw to let the caller know it failed
    }
  }

  private getWorktreePath(): string {
    return `../windmill-ephemeral-backends/${this.config.commitHash}`;
  }

  private async createWorktree(): Promise<void> {
    console.log("\n📂 Creating git worktree...");

    const worktreeBasePath = "../windmill-ephemeral-backends";
    const worktreePath = this.getWorktreePath();
    this.resources.worktreePath = worktreePath;

    // Check if worktree already exists
    try {
      const { stdout } = await execAsync("git worktree list");
      if (stdout.includes(worktreePath)) {
        console.log(`✓ Worktree already exists at ${worktreePath}`);
        return;
      }
    } catch (error) {
      // Worktree doesn't exist, we'll create it
    }

    // Create the base directory if it doesn't exist
    try {
      await execAsync(`mkdir -p ${worktreeBasePath}`);
    } catch (error) {
      // Directory might already exist
    }

    // Create the worktree
    console.log(
      `  Creating worktree at ${worktreePath} for commit ${this.config.commitHash}`
    );
    await execAsync(
      `git worktree add ${worktreePath} ${this.config.commitHash}`
    );
    console.log(`✓ Worktree created at ${worktreePath}`);
  }

  private async spawnPostgres(): Promise<void> {
    console.log("\n🐘 Spawning PostgreSQL container...");

    this.resources.dbProcess = spawn("docker", [
      "run",
      "--rm",
      "-p",
      `${this.config.dbPort}:5432`,
      "-e",
      "POSTGRES_PASSWORD=changeme",
      "-e",
      "POSTGRES_DB=windmill",
      "postgres:16",
    ]);

    // Capture container ID from first line of output
    this.resources.dbProcess.stdout.on("data", (data: Buffer) => {
      process.stdout.write(`[postgres] ${data}`);
    });

    this.resources.dbProcess.stderr.on("data", (data: Buffer) => {
      process.stderr.write(`[postgres] ${data}`);
    });

    this.resources.dbProcess.on("close", (code: number) => {
      console.log(`PostgreSQL process exited with code ${code}`);
    });
  }

  private async waitForPostgres(): Promise<void> {
    console.log("⏳ Waiting for PostgreSQL to be ready...");

    const maxAttempts = 30;
    const delayMs = 1000;

    for (let attempt = 1; attempt <= maxAttempts; attempt++) {
      try {
        await execAsync(
          `docker run --rm postgres:16 pg_isready -h host.docker.internal -p ${this.config.dbPort} -U postgres`
        );
        console.log("✓ PostgreSQL is ready");
        return;
      } catch (error) {
        if (attempt === maxAttempts) {
          throw new Error("PostgreSQL failed to start in time");
        }
        await new Promise((resolve) => setTimeout(resolve, delayMs));
      }
    }
  }

  private async buildBackend(): Promise<void> {
    console.log("\n🔨 Building backend (this may take a while)...");

    // Detect OS to use correct deno_core feature
    const isMacOS = process.platform === "darwin";

    const env = {
      ...process.env,
      SQLX_OFFLINE: "true",
      DATABASE_URL: `postgres://postgres:changeme@localhost:${this.config.dbPort}/windmill?sslmode=disable`,
    };

    const features = [
      "enterprise",
      "enterprise_saml",
      "stripe",
      "embedding",
      "parquet",
      "prometheus",
      "openidconnect",
      "cloud",
      "jemalloc",
      "agent_worker_server",
      "tantivy",
      "license",
      "http_trigger",
      "zip",
      "oauth2",
      "kafka",
      "sqs_trigger",
      "nats",
      "otel",
      "dind",
      "postgres_trigger",
      "mqtt_trigger",
      "gcp_trigger",
      "websocket",
      "smtp",
      "all_languages",
      "private",
      isMacOS ? "deno_core_mac" : "deno_core",
      "mcp",
    ].join(",");

    return new Promise((resolve, reject) => {
      const backendDir = `${this.getWorktreePath()}/backend`;
      const buildProcess = spawn(
        "cargo",
        ["build", "--features", features, "--release"],
        { cwd: backendDir, env }
      );

      buildProcess.stdout.on("data", (data) => {
        process.stdout.write(data);
      });

      buildProcess.stderr.on("data", (data) => {
        process.stderr.write(data);
      });

      buildProcess.on("close", (code) => {
        if (code === 0) {
          console.log("✓ Backend built successfully");
          resolve();
        } else {
          reject(new Error(`Build failed with code ${code}`));
        }
      });
    });
  }

  private async startBackend(): Promise<void> {
    console.log("\n🚀 Starting Windmill backend...");

    const env = {
      ...process.env,
      DATABASE_URL: `postgres://postgres:changeme@localhost:${this.config.dbPort}/windmill?sslmode=disable`,
      PORT: this.config.serverPort.toString(),
    };

    const releaseDir = `${this.getWorktreePath()}/backend/target/release`;
    this.resources.backendProcess = spawn("./windmill", [], {
      cwd: releaseDir,
      env,
    });

    this.resources.backendProcess.stdout.on("data", (data: Buffer) => {
      process.stdout.write(`[backend] ${data}`);
    });

    this.resources.backendProcess.stderr.on("data", (data: Buffer) => {
      process.stderr.write(`[backend] ${data}`);
    });

    this.resources.backendProcess.on("close", (code: number) => {
      console.log(`Backend process exited with code ${code}`);
    });

    // Give the backend a moment to start
    await new Promise((resolve) => setTimeout(resolve, 3000));
    console.log("✓ Backend started");
  }

  private async startCloudflared(): Promise<void> {
    if (process.env.SKIP_CLOUDFLARED) {
      this.config.onCloudflaredUrl?.("SKIP_CLOUDFLARED");
      return;
    }
    console.log("\n🌐 Starting Cloudflare tunnel...");

    return new Promise((resolve, reject) => {
      this.resources.cloudflaredProcess = spawn("cloudflared", [
        "tunnel",
        "--url",
        `http://localhost:${this.config.serverPort}`,
        "--config",
        "/dev/null",
      ]);

      const rl = readline.createInterface({
        input: this.resources.cloudflaredProcess.stdout,
      });

      rl.on("line", (line: string) => {
        console.log(`[cloudflared] ${line}`);
      });

      this.resources.cloudflaredProcess.stderr.on("data", (data: Buffer) => {
        process.stderr.write(`[cloudflared] ${data}`);
        const line = data.toString();
        const match = line.match(/https:\/\/([a-z0-9-]+\.trycloudflare\.com)/);
        if (match) {
          this.resources.tunnelUrl = match[1];
          this.config.onCloudflaredUrl?.(this.resources.tunnelUrl);
          resolve();
        }
      });

      this.resources.cloudflaredProcess.on("close", (code: number) => {
        console.log(`Cloudflared process exited with code ${code}`);
      });

      // Timeout if we can't find the URL in 30 seconds
      setTimeout(() => {
        if (!this.resources.tunnelUrl) {
          reject(new Error("Failed to extract Cloudflare tunnel URL"));
        }
      }, 30000);
    });
  }

  async cleanup(): Promise<void> {
    console.log("\n🧹 Cleaning up resources...");

    // Ensure cleanup doesn't throw errors to prevent cascading failures
    const errors: string[] = [];

    // Kill backend process
    if (this.resources.backendProcess) {
      console.log("  Stopping backend...");
      try {
        this.resources.backendProcess.removeAllListeners(); // Remove exit handler to prevent re-entry
        this.resources.backendProcess.kill("SIGTERM");
        // Give it a moment to gracefully shutdown
        await new Promise((resolve) => setTimeout(resolve, 1000));
        // Force kill if still running
        try {
          this.resources.backendProcess.kill("SIGKILL");
        } catch (e) {
          // Already dead
        }
        console.log("  ✓ Backend stopped");
      } catch (error) {
        errors.push(`Failed to stop backend: ${error}`);
      }
    }

    // Kill cloudflared process
    if (this.resources.cloudflaredProcess) {
      console.log("  Stopping cloudflared...");
      try {
        this.resources.cloudflaredProcess.removeAllListeners();
        this.resources.cloudflaredProcess.kill("SIGTERM");
        await new Promise((resolve) => setTimeout(resolve, 1000));
        try {
          this.resources.cloudflaredProcess.kill("SIGKILL");
        } catch (e) {
          // Already dead
        }
        console.log("  ✓ Cloudflared stopped");
      } catch (error) {
        errors.push(`Failed to stop cloudflared: ${error}`);
      }
    }

    // Kill PostgreSQL process
    if (this.resources.dbProcess) {
      console.log("  Stopping PostgreSQL container...");
      try {
        this.resources.dbProcess.removeAllListeners();
        this.resources.dbProcess.kill("SIGTERM");
        await new Promise((resolve) => setTimeout(resolve, 1000));
        try {
          this.resources.dbProcess.kill("SIGKILL");
        } catch (e) {
          // Already dead
        }
        console.log("  ✓ PostgreSQL container stopped");
      } catch (error) {
        errors.push(`Failed to stop PostgreSQL: ${error}`);
      }
    }

    // Remove git worktree
    if (this.resources.worktreePath) {
      console.log("  Removing git worktree...");
      try {
        await execAsync(
          `git worktree remove ${this.resources.worktreePath} --force`
        );
        console.log("  ✓ Git worktree removed");
      } catch (error) {
        errors.push(`Failed to remove git worktree: ${error}`);
      }
    }

    if (errors.length > 0) {
      console.error("⚠️  Cleanup completed with errors:");
      errors.forEach((err) => console.error(`  - ${err}`));
    } else {
      console.log("✅ Cleanup complete");
    }

    // Always call onCleanup callback regardless of errors
    try {
      this.config.onCleanup?.();
    } catch (error) {
      console.error("  Error in cleanup callback:", error);
    }
  }
}
