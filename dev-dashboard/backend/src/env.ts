/** Read key=value pairs from a worktree's .env.local file. */
export function readEnvLocal(wtDir: string): Record<string, string> {
  try {
    const content = Bun.spawnSync(["cat", `${wtDir}/.env.local`], { stdout: "pipe" });
    const text = new TextDecoder().decode(content.stdout).trim();
    const env: Record<string, string> = {};
    for (const line of text.split("\n")) {
      const match = line.match(/^(\w+)=(.*)$/);
      if (match) env[match[1]] = match[2];
    }
    return env;
  } catch {
    return {};
  }
}
