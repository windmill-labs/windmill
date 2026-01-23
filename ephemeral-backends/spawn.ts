#!/usr/bin/env node

import { spawn, exec } from "child_process";
import { promisify } from "util";
import * as readline from "readline";

const execAsync = promisify(exec);

export interface Config {
  dbPort: number;
  serverPort: number;
  skipBuild: boolean;
  onCloudflaredUrl?: (url: string) => void;
  onCleanup?: () => void;
}

interface SpawnedResources {
  dbContainerId: string;
  backendProcess?: any;
  cloudflaredProcess?: any;
  tunnelUrl?: string;
}

const DEFAULT_CONFIG: Config = {
  dbPort: 5432,
  serverPort: 8000,
  skipBuild: false,
};

export class EphemeralBackend {
  private config: Config;
  private resources: SpawnedResources = { dbContainerId: "" };

  getDbPort(): number {
    return this.config.dbPort;
  }
  getServerPort(): number {
    return this.config.serverPort;
  }

  constructor(config: Partial<Config> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config };
  }

  async spawn(): Promise<void> {
    try {
      console.log("🚀 Starting ephemeral backend...");
      console.log(`📊 Database port: ${this.config.dbPort}`);
      console.log(`🌐 Server port: ${this.config.serverPort}`);

      await this.startCloudflared();
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
      console.log("\n💡 Press Ctrl+C to stop and cleanup...");

      // Keep the process running indefinitely
      await new Promise(() => {}); // Never resolves
    } catch (error) {
      console.error("❌ Error spawning ephemeral backend:", error);
      await this.cleanup();
    }
  }

  private async spawnPostgres(): Promise<void> {
    console.log("\n🐘 Spawning PostgreSQL container...");

    const { stdout } = await execAsync(
      `docker run --rm -d -p ${this.config.dbPort}:5432 ` +
        `-e POSTGRES_PASSWORD=changeme ` +
        `-e POSTGRES_DB=windmill ` +
        `postgres:16`
    );

    this.resources.dbContainerId = stdout.trim();
    console.log(
      `✓ PostgreSQL container started: ${this.resources.dbContainerId.substring(
        0,
        12
      )}`
    );
  }

  private async waitForPostgres(): Promise<void> {
    console.log("⏳ Waiting for PostgreSQL to be ready...");

    const maxAttempts = 30;
    const delayMs = 1000;

    for (let attempt = 1; attempt <= maxAttempts; attempt++) {
      try {
        await execAsync(
          `docker exec ${this.resources.dbContainerId} pg_isready -U postgres`
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
      const buildProcess = spawn(
        "cargo",
        ["build", "--features", features, "--release"],
        { cwd: "./backend", env }
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

    this.resources.backendProcess = spawn("./windmill", [], {
      cwd: "./backend/target/release",
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
    console.log("\n🌐 Starting Cloudflare tunnel...");
    if (process.env.SKIP_CLOUDFLARED) {
      this.config.onCloudflaredUrl?.("SKIP_CLOUDFLARED");
      return;
    }

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

    // Kill backend process
    if (this.resources.backendProcess) {
      console.log("  Stopping backend...");
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
      console.log("  Stopping cloudflared...");
      try {
        this.resources.cloudflaredProcess.kill("SIGTERM");
        await new Promise((resolve) => setTimeout(resolve, 1000));
        this.resources.cloudflaredProcess.kill("SIGKILL");
      } catch (error) {
        // Process might already be dead
      }
    }

    // Stop and remove PostgreSQL container
    if (this.resources.dbContainerId) {
      console.log("  Stopping PostgreSQL container...");
      try {
        await execAsync(`docker stop ${this.resources.dbContainerId}`);
        console.log("  ✓ PostgreSQL container stopped");
      } catch (error) {
        console.error("  Failed to stop PostgreSQL container:", error);
      }
    }

    this.config.onCleanup?.();

    console.log("✅ Cleanup complete");
  }
}
