#!/usr/bin/env node

import { spawn } from "child_process";
import * as readline from "readline";
import sodium from "libsodium-wrappers-sumo";
import { EphemeralBackend } from "./spawn";
import { WorktreePool } from "./worktree-pool";
import { Logger } from "./logger";
import { readFileSync, existsSync } from "fs";

process.on("unhandledRejection", (err) => {
  console.error("UNHANDLED PROMISE:", err);
});

process.on("uncaughtException", (err) => {
  console.error("UNCAUGHT EXCEPTION:", err);
});

const githubToken = process.env.GITHUB_TOKEN;
if (!githubToken) {
  console.log("⚠️  GITHUB_TOKEN environment variable not set");
  console.log("\n📝 Set a GitHub token with 'secrets' scope:");
  console.log("   export GITHUB_TOKEN=github_pat_...");
  process.exit(1);
}

if (!process.env.GIT_EE_DEPLOY_KEY_FILE) {
  console.log("⚠️  GIT_EE_DEPLOY_KEY_FILE environment variable not set");
  console.log("\n📝 Set a read-only SSH deploy key:");
  console.log("   export GIT_EE_DEPLOY_KEY_FILE=/home/...");
  process.exit(1);
}

const MANAGER_PORT = 8001;
const BACKEND_TIMEOUT_MS = 60 * 60 * 1000; // 1 hour in milliseconds

interface BackendInfo {
  backend: EphemeralBackend;
  timeoutId: NodeJS.Timeout;
  createdAt: Date;
}

interface ManagerResources {
  cloudflaredProcess?: any;
  tunnelUrl?: string;
  ephemeralBackends: Map<string, BackendInfo>;
  worktreePool?: WorktreePool;
}

class EphemeralBackendManager {
  private resources: ManagerResources = {
    ephemeralBackends: new Map(),
  };
  private server?: any;

  async start(): Promise<void> {
    // Setup cleanup handlers early
    process.on("SIGINT", () => this.cleanup());
    process.on("SIGTERM", () => this.cleanup());

    try {
      console.log("🎛️  Starting Ephemeral Backend Manager...");
      console.log(`📊 Manager port: ${MANAGER_PORT}`);

      // Initialize the worktree pool
      this.resources.worktreePool = new WorktreePool();
      await this.resources.worktreePool.initialize();

      await this.startHttpServer();
      if (!process.env.SKIP_CLOUDFLARED) await this.startCloudflared();
      if (!process.env.SKIP_SET_GH_SECRET) await this.updateGitHubSecret();

      console.log("\n✅ Manager is ready!");
      console.log(`📍 Tunnel URL: ${this.resources.tunnelUrl}`);

      console.log("\n💡 Press Ctrl+C to stop...");

      // Keep the process running indefinitely
      await new Promise(() => {}); // Never resolves
    } catch (error) {
      console.error("❌ Error starting manager:", error);
      await this.cleanup();
      process.exit(1);
    }
  }

  private async startHttpServer(): Promise<void> {
    const self = this;
    console.log("\n🌐 Starting HTTP server...");

    return new Promise((resolve) => {
      // Use Bun's built-in HTTP server
      this.server = Bun.serve({
        port: MANAGER_PORT,
        idleTimeout: 30,
        async fetch(req) {
          const url = new URL(req.url);

          // Health check endpoint
          if (url.pathname === "/health") {
            return new Response(
              JSON.stringify({
                status: "ok",
                timestamp: new Date().toISOString(),
              }),
              { headers: { "Content-Type": "application/json" } }
            );
          }

          // Status endpoint - shows all running backends and worktree pool stats
          if (url.pathname === "/status") {
            const backends = Array.from(
              self.resources.ephemeralBackends.entries()
            ).map(([commitHash, backendInfo]) => {
              const now = new Date();
              const timeoutAt = new Date(
                backendInfo.createdAt.getTime() + BACKEND_TIMEOUT_MS
              );
              const timeRemainingMs = timeoutAt.getTime() - now.getTime();
              const timeRemainingMinutes = Math.floor(
                timeRemainingMs / 1000 / 60
              );

              return {
                commitHash,
                shortHash: commitHash.substring(0, 8),
                createdAt: backendInfo.createdAt.toISOString(),
                timeoutAt: timeoutAt.toISOString(),
                timeRemainingMinutes,
                serverPort: backendInfo.backend.getServerPort(),
                dbPort: backendInfo.backend.getDbPort(),
              };
            });

            const worktreePoolStats =
              self.resources.worktreePool?.getStats() || {
                total: 0,
                inUse: 0,
                available: 0,
              };

            return new Response(
              JSON.stringify({
                activeBackends: backends.length,
                backends,
                worktreePool: worktreePoolStats,
                timestamp: new Date().toISOString(),
              }),
              { headers: { "Content-Type": "application/json" } }
            );
          }

          // Match /logs/{commit_hash} - serve log files
          const logsMatch = url.pathname.match(/^\/logs\/([a-f0-9]+)$/);
          if (logsMatch && req.method === "GET") {
            const commitHash = logsMatch[1];

            // Validate commit hash format (7-40 hex characters)
            if (commitHash.length < 7 || commitHash.length > 40) {
              return new Response("Invalid commit hash", { status: 400 });
            }

            try {
              // Use the Logger class's secure path resolution
              const logFilePath = Logger.getLogFilePathForCommit(commitHash);

              // Check if file exists
              if (!existsSync(logFilePath)) {
                return new Response("Log file not found", { status: 404 });
              }

              // Read the log file
              const logContent = readFileSync(logFilePath, "utf-8");

              return new Response(logContent, {
                headers: {
                  "Content-Type": "text/plain; charset=utf-8",
                  "X-Commit-Hash": commitHash,
                },
              });
            } catch (error: any) {
              console.error(`Error reading log file for ${commitHash}:`, error);
              return new Response(
                `Error reading log file: ${error.message}`,
                { status: 500 }
              );
            }
          }

          // Match /spawn/{commit_hash}
          const spawnMatch = url.pathname.match(/^\/spawn\/([a-f0-9]+)$/);
          if (spawnMatch && req.method === "POST") {
            let body = await req.json();
            if (typeof body !== "object")
              return new Response("Invalid JSON body", { status: 400 });
            let resumeUrl = body?.resume_url;
            if (typeof resumeUrl !== "string")
              return new Response("Invalid resume_url", { status: 400 });
            let cancelUrl = body?.cancel_url;
            if (typeof cancelUrl !== "string")
              return new Response("Invalid cancel_url", { status: 400 });
            const commitHash = spawnMatch[1];
            console.log(
              `\n🔹 Received request to spawn ephemeral backend for commit: ${commitHash}`
            );
            if (self.resources.ephemeralBackends.has(commitHash)) {
              throw new Error(`Backend ${commitHash} is already running`);
            }

            if (!self.resources.worktreePool) {
              throw new Error("Worktree pool not initialized");
            }

            const tunnelUrl = await new Promise<string>((res, err) => {
              const ephemeralBackend = new EphemeralBackend({
                dbPort: self.findFreeDbPorts(),
                serverPort: self.findFreeServerPorts(),
                skipBuild: !!process.env.SKIP_BACKEND_BUILD,
                commitHash: commitHash,
                worktreePool: self.resources.worktreePool!,
                onCloudflaredUrl: (url) => (res(url), clearTimeout(timeout)),
                onCleanup: () => {
                  const backendInfo =
                    self.resources.ephemeralBackends.get(commitHash);
                  if (backendInfo) {
                    clearTimeout(backendInfo.timeoutId);
                    self.resources.ephemeralBackends.delete(commitHash);
                  }
                },
              });
              function onError(e: any) {
                ephemeralBackend.cleanup().catch(() => {
                  console.error(
                    `Failed to cleanup backend for commit ${commitHash}`
                  );
                });
                clearTimeout(timeout);
                fetch(resumeUrl, {
                  // Cancel URL doesn't show any relevant info, use resume URL
                  method: "POST",
                  headers: { "Content-Type": "application/json" },
                  body: JSON.stringify({
                    status: "error",
                    commitHash,
                    error: e.message,
                  }),
                }).catch((e) => {
                  console.error(
                    `Failed to notify cancel URL for commit ${commitHash}:`,
                    e
                  );
                });
              }
              const timeout = setTimeout(() => {
                onError(new Error("Timeout waiting for backend URL"));
              }, 20000);
              try {
                ephemeralBackend
                  .spawn()
                  .then(({ tunnelUrl }) => {
                    if (resumeUrl) {
                      fetch(resumeUrl, {
                        method: "POST",
                        headers: { "Content-Type": "application/json" },
                        body: JSON.stringify({
                          status: "ready",
                          commitHash,
                          tunnelUrl: `https://${tunnelUrl}`,
                        }),
                      }).catch((e) => {
                        onError(e);
                      });
                    }
                  })
                  .catch((e) => onError(e));

                // Set up 1-hour timeout for automatic cleanup
                const cleanupTimeoutId = setTimeout(async () => {
                  console.log(
                    `\n⏰ Backend ${commitHash} has reached 1-hour timeout, cleaning up...`
                  );
                  try {
                    await ephemeralBackend.cleanup();
                    self.resources.ephemeralBackends.delete(commitHash);
                    console.log(
                      `✓ Backend ${commitHash} cleaned up after timeout`
                    );
                  } catch (error) {
                    console.error(
                      `❌ Failed to cleanup backend ${commitHash} after timeout:`,
                      error
                    );
                  }
                }, BACKEND_TIMEOUT_MS);

                self.resources.ephemeralBackends.set(commitHash, {
                  backend: ephemeralBackend,
                  timeoutId: cleanupTimeoutId,
                  createdAt: new Date(),
                });
              } catch (e) {
                onError(e);
              }
            });

            return new Response(
              JSON.stringify({
                tunnelUrl,
                timestamp: new Date().toISOString(),
              }),
              { headers: { "Content-Type": "application/json" }, status: 202 }
            );
          }

          // Default 404
          return new Response("Not Found", { status: 404 });
        },
      });

      console.log(`✓ HTTP server listening on port ${MANAGER_PORT}`);
      resolve();
    });
  }

  private async startCloudflared(): Promise<void> {
    console.log("\n🌐 Starting Cloudflare tunnel for manager...");

    return new Promise((resolve, reject) => {
      this.resources.cloudflaredProcess = spawn("cloudflared", [
        "tunnel",
        "--url",
        `http://localhost:${MANAGER_PORT}`,
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
          console.log(`✓ Tunnel URL extracted: ${this.resources.tunnelUrl}`);
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

  private async updateGitHubSecret(): Promise<void> {
    console.log("\n🔐 Updating GitHub Actions secret...");

    if (!this.resources.tunnelUrl) {
      console.error("❌ No tunnel URL available to update secret");
      return;
    }

    const fullUrl = `https://${this.resources.tunnelUrl}`;
    const repo = "windmill-labs/windmill";
    const secretName = "EPHEMERAL_BACKEND_QUEUE_URL";

    try {
      // First, get the repository public key for encrypting the secret
      console.log("  Fetching repository public key...");
      const keyResponse = await fetch(
        `https://api.github.com/repos/${repo}/actions/secrets/public-key`,
        {
          headers: {
            Authorization: `Bearer ${githubToken}`,
            Accept: "application/vnd.github+json",
            "X-GitHub-Api-Version": "2022-11-28",
          },
        }
      );

      if (!keyResponse.ok) {
        throw new Error(
          `Failed to fetch public key: ${keyResponse.statusText}`
        );
      }

      const { key, key_id } = await keyResponse.json();

      // Encrypt the secret using libsodium (via tweetnacl for Bun compatibility)
      console.log("  Encrypting secret value...");
      await sodium.ready;

      const messageBytes = new TextEncoder().encode(fullUrl);
      const keyBytes = sodium.from_base64(key, sodium.base64_variants.ORIGINAL);
      const encryptedBytes = sodium.crypto_box_seal(messageBytes, keyBytes);
      const encryptedValue = sodium.to_base64(
        encryptedBytes,
        sodium.base64_variants.ORIGINAL
      );

      // Update the secret
      console.log("  Updating secret...");
      const updateResponse = await fetch(
        `https://api.github.com/repos/${repo}/actions/secrets/${secretName}`,
        {
          method: "PUT",
          headers: {
            Authorization: `Bearer ${githubToken}`,
            Accept: "application/vnd.github+json",
            "X-GitHub-Api-Version": "2022-11-28",
            "Content-Type": "application/json",
          },
          body: JSON.stringify({
            encrypted_value: encryptedValue,
            key_id: key_id,
          }),
        }
      );

      if (!updateResponse.ok) {
        const errorText = await updateResponse.text();
        throw new Error(
          `Failed to update secret: ${updateResponse.statusText} - ${errorText}`
        );
      }

      console.log(`✓ GitHub secret updated successfully!`);
      console.log(`  Repository: ${repo}`);
      console.log(`  Secret: ${secretName}`);
      console.log(`  Value: ${fullUrl}`);
    } catch (error: any) {
      console.error("❌ Failed to update GitHub secret:", error.message);
    }
  }

  private async deleteGitHubSecret(): Promise<void> {
    const repo = "windmill-labs/windmill";
    const secretName = "EPHEMERAL_BACKEND_QUEUE_URL";

    try {
      console.log("  Deleting GitHub Actions secret...");
      const deleteResponse = await fetch(
        `https://api.github.com/repos/${repo}/actions/secrets/${secretName}`,
        {
          method: "DELETE",
          headers: {
            Authorization: `Bearer ${githubToken}`,
            Accept: "application/vnd.github+json",
            "X-GitHub-Api-Version": "2022-11-28",
          },
        }
      );

      if (!deleteResponse.ok) {
        // 404 is acceptable - secret might not exist
        if (deleteResponse.status === 404) {
          console.log(
            `  ✓ Secret ${secretName} does not exist (already deleted)`
          );
          return;
        }
        const errorText = await deleteResponse.text();
        throw new Error(
          `Failed to delete secret: ${deleteResponse.statusText} - ${errorText}`
        );
      }

      console.log(`✓ GitHub secret deleted successfully!`);
      console.log(`  Repository: ${repo}`);
      console.log(`  Secret: ${secretName}`);
    } catch (error: any) {
      console.error("❌ Failed to delete GitHub secret:", error.message);
    }
  }

  isCleaningUp: boolean = false;
  private async cleanup(): Promise<void> {
    if (this.isCleaningUp) return;
    this.isCleaningUp = true;
    console.log("\n🧹 Cleaning up manager resources...");

    // Delete GitHub secret
    if (!process.env.SKIP_SET_GH_SECRET) {
      console.log("  Deleting GitHub Actions secret...");
      try {
        await this.deleteGitHubSecret();
      } catch (error) {
        console.error("  Failed to delete GitHub secret:", error);
      }
    }

    // Stop HTTP server
    if (this.server) {
      console.log("  Stopping HTTP server...");
      try {
        this.server.stop();
      } catch (error) {
        console.error("  Failed to stop HTTP server:", error);
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

    for (const [commitHash, backendInfo] of this.resources.ephemeralBackends) {
      const hash = commitHash.substring(0, 8);
      console.log(
        `  Cleaning up ephemeral backend ${hash} on port ${backendInfo.backend.getServerPort()}...`
      );
      try {
        clearTimeout(backendInfo.timeoutId); // Clear the timeout before cleanup
        await backendInfo.backend.cleanup();
      } catch (error) {
        console.error(
          `  Failed to clean up backend ${hash} on port ${backendInfo.backend.getServerPort()}:`,
          error
        );
      }
    }

    console.log("✅ Cleanup complete");
    process.exit(0);
  }

  private findFreeDbPorts(): number {
    const minPort = 5433;
    for (let port = minPort; port < minPort + 100; port++) {
      if (
        ![...this.resources.ephemeralBackends.values()].some(
          (backendInfo) => port === backendInfo.backend.getDbPort()
        )
      ) {
        return port;
      }
    }
    throw new Error("No free DB ports available");
  }

  private findFreeServerPorts(): number {
    const minPort = 8002;
    for (let port = minPort; port < minPort + 100; port++) {
      if (
        ![...this.resources.ephemeralBackends.values()].some(
          (backendInfo) => port === backendInfo.backend.getServerPort()
        )
      ) {
        return port;
      }
    }
    throw new Error("No free server ports available");
  }
}

// Main execution
async function main() {
  const manager = new EphemeralBackendManager();
  await manager.start();
}

main().catch((error) => {
  console.error("Fatal error:", error);
  process.exit(1);
});
