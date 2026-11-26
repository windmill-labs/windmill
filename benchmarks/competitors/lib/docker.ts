/**
 * Docker Compose lifecycle helpers.
 * Wraps `docker compose` CLI for starting/stopping competitor stacks.
 */

import { dirname } from "https://deno.land/std@0.224.0/path/mod.ts";

async function run(
  args: string[],
  cwd?: string,
): Promise<{ code: number; stdout: string; stderr: string }> {
  const cmd = new Deno.Command("docker", { args, cwd, stdout: "piped", stderr: "piped" });
  const output = await cmd.output();
  return {
    code: output.code,
    stdout: new TextDecoder().decode(output.stdout),
    stderr: new TextDecoder().decode(output.stderr),
  };
}

export async function composeUp(composeFile: string): Promise<void> {
  const cwd = dirname(composeFile);
  const file = composeFile.split("/").pop()!;

  // Force-remove any lingering containers from a previous run
  await run(["compose", "-f", file, "down", "-v", "--remove-orphans"], cwd);

  const { code, stderr } = await run(
    ["compose", "-f", file, "up", "-d", "--force-recreate"],
    cwd,
  );
  if (code !== 0) {
    throw new Error(`docker compose up failed for ${composeFile}:\n${stderr}`);
  }
}

export async function composeDown(composeFile: string): Promise<void> {
  const cwd = dirname(composeFile);
  const file = composeFile.split("/").pop()!;
  const { code, stderr } = await run(
    ["compose", "-f", file, "down", "-v", "--remove-orphans"],
    cwd,
  );
  if (code !== 0) {
    console.error(`docker compose down warning for ${composeFile}:\n${stderr}`);
  }
}

export async function composeLogs(
  composeFile: string,
  service?: string,
): Promise<string> {
  const cwd = dirname(composeFile);
  const file = composeFile.split("/").pop()!;
  const args = ["compose", "-f", file, "logs", "--tail=100"];
  if (service) args.push(service);
  const { stdout } = await run(args, cwd);
  return stdout;
}

/**
 * Poll an HTTP endpoint until it returns 2xx or we exceed maxRetries.
 */
export async function waitForHealth(
  url: string,
  { maxRetries = 30, intervalMs = 2000 } = {},
): Promise<void> {
  for (let i = 0; i < maxRetries; i++) {
    try {
      const resp = await fetch(url);
      if (resp.ok) {
        // drain body
        await resp.text();
        return;
      }
      await resp.body?.cancel();
    } catch (_) {
      // connection refused, retry
    }
    await new Promise((r) => setTimeout(r, intervalMs));
  }
  throw new Error(
    `Health check failed after ${maxRetries} retries: ${url}`,
  );
}
