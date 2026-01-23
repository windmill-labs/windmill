#!/usr/bin/env node

import { spawn } from "child_process";
import * as readline from "readline";
import sodium from "libsodium-wrappers-sumo";

const githubToken = process.env.GITHUB_TOKEN;
if (!githubToken) {
  console.log("⚠️  GITHUB_TOKEN environment variable not set");
  console.log("\n📝 Set a GitHub token with 'secrets' scope:");
  console.log("   export GITHUB_TOKEN=github_pat_...");
  process.exit(1);
}

const MANAGER_PORT = 8001;

interface ManagerResources {
  cloudflaredProcess?: any;
  tunnelUrl?: string;
}

class EphemeralBackendManager {
  private resources: ManagerResources = {};
  private server?: any;

  async start(): Promise<void> {
    // Setup cleanup handlers early
    process.on("SIGINT", () => this.cleanup());
    process.on("SIGTERM", () => this.cleanup());

    try {
      console.log("🎛️  Starting Ephemeral Backend Manager...");
      console.log(`📊 Manager port: ${MANAGER_PORT}`);

      await this.startHttpServer();
      await this.startCloudflared();

      console.log("\n✅ Manager is ready!");
      console.log(`📍 Tunnel URL: ${this.resources.tunnelUrl}`);

      // Automatically update GitHub secret
      await this.updateGitHubSecret();

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
    console.log("\n🌐 Starting HTTP server...");

    return new Promise((resolve) => {
      // Use Bun's built-in HTTP server
      this.server = Bun.serve({
        port: MANAGER_PORT,
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

          // Spawn backend endpoint
          if (url.pathname === "/spawn" && req.method === "POST") {
            return new Response(
              JSON.stringify({
                message: "Spawn endpoint received (implementation pending)",
                timestamp: new Date().toISOString(),
              }),
              {
                headers: { "Content-Type": "application/json" },
                status: 202, // Accepted
              }
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

  private async cleanup(): Promise<void> {
    console.log("\n🧹 Cleaning up manager resources...");

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

    console.log("✅ Cleanup complete");
    process.exit(0);
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
