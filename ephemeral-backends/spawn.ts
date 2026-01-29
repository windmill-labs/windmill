#!/usr/bin/env node

import { spawn, exec } from "child_process";
import { promisify } from "util";
import * as readline from "readline";
import path from "path";
import { WorktreePool, WorktreeInfo } from "./worktree-pool";
import { Logger } from "./logger";

const execAsync = promisify(exec);

export interface Config {
  dbPort: number;
  serverPort: number;
  skipBuild: boolean;
  commitHash: string;
  worktreePool: WorktreePool;
  onCloudflaredUrl?: (url: string) => void;
  onCleanup?: () => void;
}

interface SpawnedResources {
  dbContainerId: string;
  dbProcess?: any;
  backendProcess?: any;
  cloudflaredProcess?: any;
  tunnelUrl?: string;
  worktree?: WorktreeInfo;
  eeWorktreePath?: string;
  logger?: Logger;
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
  getLogFilePath(): string | undefined {
    return this.resources.logger?.getLogFilePath();
  }

  constructor(config: Config) {
    this.config = config;
  }

  async spawn(): Promise<{
    tunnelUrl: string;
  }> {
    try {
      // Initialize logger for this ephemeral backend
      this.resources.logger = new Logger(this.config.commitHash, "backend");

      this.resources.logger.log("🚀 Starting ephemeral backend...");
      this.resources.logger.log(`📊 Database port: ${this.config.dbPort}`);
      this.resources.logger.log(`🌐 Server port: ${this.config.serverPort}`);
      this.resources.logger.log(`📌 Commit hash: ${this.config.commitHash}`);

      await this.startCloudflared();
      if (!this.resources.tunnelUrl)
        throw new Error("Cloudflare tunnel URL not available");
      await this.acquireWorktree();
      await this.setupEECode();
      await this.spawnPostgres();
      await this.waitForPostgres();
      if (!this.config.skipBuild) {
        await this.buildBackend();
      } else {
        this.resources.logger?.log(
          "\n⏭️  Skipping backend build (using existing binary)"
        );
      }
      await this.startBackend();

      // Release the worktree back to the pool now that the backend is running
      // The binary is already compiled and running, so other spawns can reuse this worktree
      if (this.resources.worktree) {
        this.resources.logger?.log(
          "\n♻️  Releasing worktree back to pool (backend is now running)..."
        );
        await this.config.worktreePool.release(this.resources.worktree.id);
        // Keep the reference for cleanup but mark it as released
        this.resources.worktree = undefined;
      }

      this.resources.logger?.log("\n✅ Ephemeral backend is ready!");
      this.resources.logger?.log(`📍 Tunnel URL: ${this.resources.tunnelUrl}`);
      this.resources.logger?.log(
        `📄 Log file: ${this.resources.logger.getLogFilePath()}`
      );

      return {
        tunnelUrl: this.resources.tunnelUrl,
      };
    } catch (error) {
      this.resources.logger?.error(
        `❌ Error spawning ephemeral backend: ${error}`
      );
      throw error;
    }
  }

  private async acquireWorktree(): Promise<void> {
    this.resources.logger?.log("\n📂 Acquiring worktree from pool...");

    // Acquire a worktree from the pool
    this.resources.worktree = await this.config.worktreePool.acquire(
      this.config.commitHash
    );

    this.resources.logger?.log(
      `✓ Worktree acquired: ${this.resources.worktree.path}`
    );
  }

  private async setupEECode(): Promise<void> {
    this.resources.logger?.log("\n🔐 Setting up Enterprise Edition code...");

    if (!this.resources.worktree) {
      throw new Error("Worktree not acquired");
    }

    const worktreePath = this.resources.worktree.path;
    const eeRefPath = `${worktreePath}/backend/ee-repo-ref.txt`;
    const eeWorktreePath = this.config.worktreePool.getEEWorktreePath(
      this.resources.worktree
    );
    this.resources.eeWorktreePath = eeWorktreePath;

    // Read the EE commit hash from ee-repo-ref.txt
    this.resources.logger?.log(
      `  Reading EE commit reference from ${eeRefPath}`
    );
    let eeCommitHash: string;
    try {
      const { stdout } = await execAsync(`cat ${eeRefPath}`);
      eeCommitHash = stdout.trim();
      if (!eeCommitHash) {
        throw new Error("ee-repo-ref.txt is empty");
      }
      this.resources.logger?.log(`  ✓ EE commit hash: ${eeCommitHash}`);
    } catch (error) {
      throw new Error(
        `Failed to read ee-repo-ref.txt: ${
          error instanceof Error ? error.message : String(error)
        }`
      );
    }

    // Remove existing EE private folder if it exists (from previous runs)
    this.resources.logger?.log(`  Cleaning up any existing EE repository...`);
    try {
      await execAsync(`rm -rf ${eeWorktreePath}`);
    } catch (error) {
      // Ignore errors if directory doesn't exist
    }

    // Clone the windmill-ee-private repo at the specific commit
    this.resources.logger?.log(
      `  Cloning windmill-ee-private at commit ${eeCommitHash}`
    );
    try {
      await execAsync(
        `git clone git@github.com:windmill-labs/windmill-ee-private.git ${eeWorktreePath}`,
        {
          env: {
            ...process.env,
            GIT_SSH_COMMAND: `ssh -i ${process.env.GIT_EE_DEPLOY_KEY_FILE} -o StrictHostKeyChecking=accept-new`,
          },
        }
      );
      this.resources.logger?.log(`  ✓ Repository cloned to ${eeWorktreePath}`);
    } catch (error) {
      throw new Error(
        `Failed to clone windmill-ee-private: ${
          error instanceof Error ? error.message : String(error)
        }`
      );
    }

    // Checkout the specific commit
    this.resources.logger?.log(`  Checking out commit ${eeCommitHash}`);
    try {
      await execAsync(`git checkout ${eeCommitHash}`, {
        cwd: eeWorktreePath,
      });
      this.resources.logger?.log(`  ✓ Checked out commit ${eeCommitHash}`);
    } catch (error) {
      throw new Error(
        `Failed to checkout EE commit: ${
          error instanceof Error ? error.message : String(error)
        }`
      );
    }

    // Run the substitute_ee_code.sh script to copy EE files
    this.resources.logger?.log(
      `  Running substitute_ee_code.sh to copy EE files`
    );
    try {
      await execAsync(`./substitute_ee_code.sh --copy -d ${eeWorktreePath}`, {
        cwd: `${worktreePath}/backend`,
      });
      this.resources.logger?.log(`  ✓ EE code substituted successfully`);
    } catch (error) {
      throw new Error(
        `Failed to substitute EE code: ${
          error instanceof Error ? error.message : String(error)
        }`
      );
    }

    this.resources.logger?.log("✓ Enterprise Edition code setup complete");
  }

  private async spawnPostgres(): Promise<void> {
    this.resources.logger?.log("\n🐘 Spawning PostgreSQL container...");

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

    // Capture and log postgres stdout
    this.resources.dbProcess.stdout.on("data", (data: Buffer) => {
      const output = data.toString().trim();
      if (output) {
        this.resources.logger?.log(`[postgres] ${output}`);
      }
    });

    // Capture and log postgres stderr
    this.resources.dbProcess.stderr.on("data", (data: Buffer) => {
      const output = data.toString().trim();
      if (output) {
        this.resources.logger?.error(`[postgres] ${output}`);
      }
    });

    this.resources.dbProcess.on("close", (code: number) => {
      this.resources.logger?.log(`PostgreSQL process exited with code ${code}`);
    });
  }

  private async waitForPostgres(): Promise<void> {
    this.resources.logger?.log("⏳ Waiting for PostgreSQL to be ready...");

    const maxAttempts = 30;
    const delayMs = 1000;

    // Determine the host to connect to
    // On Linux, we need to use the host's IP or localhost
    // On macOS/Windows, host.docker.internal works
    const isLinux = process.platform === "linux";
    const dbHost = isLinux ? "172.17.0.1" : "host.docker.internal";

    for (let attempt = 1; attempt <= maxAttempts; attempt++) {
      try {
        await execAsync(
          `docker run --rm postgres:16 pg_isready -h ${dbHost} -p ${this.config.dbPort} -U postgres`
        );
        this.resources.logger?.log("✓ PostgreSQL is ready");
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
    this.resources.logger?.log(
      "\n🔨 Building backend (this may take a while)..."
    );

    if (!this.resources.worktree) {
      throw new Error("Worktree not acquired");
    }

    // Detect OS to use correct deno_core feature
    const isMacOS = process.platform === "darwin";

    const env = { ...process.env, SQLX_OFFLINE: "true" };

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
      const backendDir = `${this.resources.worktree?.path}/backend`;
      const buildProcess = spawn(
        "cargo",
        ["build", "--features", features, "--release"],
        { cwd: backendDir, env }
      );

      buildProcess.stdout.on("data", (data) => {
        const output = data.toString().trim();
        if (output) {
          this.resources.logger?.log(`[build] ${output}`);
        }
      });

      buildProcess.stderr.on("data", (data) => {
        const output = data.toString().trim();
        if (output) {
          this.resources.logger?.error(`[build] ${output}`);
        }
      });

      buildProcess.on("close", (code) => {
        if (code === 0) {
          this.resources.logger?.log("✓ Backend built successfully");
          resolve();
        } else {
          this.resources.logger?.error(`Build failed with code ${code}`);
          reject(new Error(`Build failed with code ${code}`));
        }
      });
    });
  }

  private async startBackend(): Promise<void> {
    this.resources.logger?.log("\n🚀 Starting Windmill backend...");

    if (!this.resources.worktree) {
      throw new Error("Worktree not acquired");
    }

    const env = {
      ...process.env,
      DATABASE_URL: `postgres://postgres:changeme@localhost:${this.config.dbPort}/windmill?sslmode=disable`,
      PORT: this.config.serverPort.toString(),
    };

    const releaseDir = `${this.resources.worktree.path}/backend/target/release`;
    this.resources.backendProcess = spawn("./windmill", [], {
      cwd: releaseDir,
      env,
    });

    // Capture and log backend stdout
    this.resources.backendProcess.stdout.on("data", (data: Buffer) => {
      const output = data.toString().trim();
      if (output) {
        this.resources.logger?.log(`[backend] ${output}`);
      }
    });

    // Capture and log backend stderr
    this.resources.backendProcess.stderr.on("data", (data: Buffer) => {
      const output = data.toString().trim();
      if (output) {
        this.resources.logger?.error(`[backend] ${output}`);
      }
    });

    this.resources.backendProcess.on("close", (code: number) => {
      this.resources.logger?.log(`Backend process exited with code ${code}`);
    });

    // Give the backend a moment to start
    await new Promise((resolve) => setTimeout(resolve, 3000));
    this.resources.logger?.log("✓ Backend started");
  }

  private async startCloudflared(): Promise<void> {
    if (process.env.SKIP_CLOUDFLARED) {
      this.config.onCloudflaredUrl?.("SKIP_CLOUDFLARED");
      return;
    }
    this.resources.logger?.log("\n🌐 Starting Cloudflare tunnel...");

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
        if (line.trim()) {
          this.resources.logger?.log(`[cloudflared] ${line}`);
        }
      });

      this.resources.cloudflaredProcess.stderr.on("data", (data: Buffer) => {
        const line = data.toString();
        if (line.trim()) {
          this.resources.logger?.error(`[cloudflared] ${line.trim()}`);
        }
        const match = line.match(/https:\/\/([a-z0-9-]+\.trycloudflare\.com)/);
        if (match) {
          this.resources.tunnelUrl = match[1];
          this.resources.logger?.log(
            `✓ Tunnel URL: ${this.resources.tunnelUrl}`
          );
          this.config.onCloudflaredUrl?.(this.resources.tunnelUrl);
          resolve();
        }
      });

      this.resources.cloudflaredProcess.on("close", (code: number) => {
        this.resources.logger?.log(
          `Cloudflared process exited with code ${code}`
        );
      });

      // Timeout if we can't find the URL in 30 seconds
      setTimeout(() => {
        if (!this.resources.tunnelUrl) {
          this.resources.logger?.error(
            "Failed to extract Cloudflare tunnel URL (timeout)"
          );
          reject(new Error("Failed to extract Cloudflare tunnel URL"));
        }
      }, 30000);
    });
  }

  async cleanup(): Promise<void> {
    this.resources.logger?.log("\n🧹 Cleaning up resources...");

    // Kill backend process
    if (this.resources.backendProcess) {
      this.resources.logger?.log("  Stopping backend...");
      try {
        this.resources.backendProcess.kill("SIGTERM");
        // Give it a moment to gracefully shutdown
        await new Promise((resolve) => setTimeout(resolve, 1000));
        // Force kill if still running
        this.resources.backendProcess.kill("SIGKILL");
      } catch (error) {
        // Process might already be dead
      }
    }

    // Kill cloudflared process
    if (this.resources.cloudflaredProcess) {
      this.resources.logger?.log("  Stopping cloudflared...");
      try {
        this.resources.cloudflaredProcess.kill("SIGTERM");
        await new Promise((resolve) => setTimeout(resolve, 1000));
        this.resources.cloudflaredProcess.kill("SIGKILL");
      } catch (error) {
        // Process might already be dead
      }
    }

    // Kill PostgreSQL process
    if (this.resources.dbProcess) {
      this.resources.logger?.log("  Stopping PostgreSQL container...");
      try {
        this.resources.dbProcess.kill("SIGTERM");
        await new Promise((resolve) => setTimeout(resolve, 1000));
        this.resources.dbProcess.kill("SIGKILL");
        this.resources.logger?.log("  ✓ PostgreSQL container stopped");
      } catch (error) {
        console.error("  Failed to stop PostgreSQL container:", error);
      }
    }

    // Remove EE private repository clone
    if (this.resources.eeWorktreePath) {
      this.resources.logger?.log("  Removing EE private repository clone...");
      try {
        await execAsync(`rm -rf ${this.resources.eeWorktreePath}`);
        this.resources.logger?.log("  ✓ EE private repository clone removed");
      } catch (error) {
        console.error("  Failed to remove EE private repository clone:", error);
      }
    }

    // Release git worktree back to pool (do not delete it)
    // Note: worktree might already be released if backend started successfully
    if (this.resources.worktree) {
      this.resources.logger?.log("  Releasing worktree back to pool...");
      try {
        await this.config.worktreePool.release(this.resources.worktree.id);
        this.resources.logger?.log("  ✓ Worktree released for reuse");
      } catch (error) {
        console.error("  Failed to release worktree:", error);
      }
    } else {
      this.resources.logger?.log("  ✓ Worktree already released");
    }

    this.config.onCleanup?.();

    this.resources.logger?.log("✅ Cleanup complete");
  }
}
