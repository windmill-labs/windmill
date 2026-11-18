import { describe, expect, it } from "bun:test";
import { chmod, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { runCodex, toToolInvocations, unwrapShellCommand } from "./codex";

describe("unwrapShellCommand", () => {
  it("strips the login-shell wrapper Codex reports", () => {
    expect(unwrapShellCommand(`/usr/bin/zsh -lc 'cat .agents/skills/x/SKILL.md'`)).toBe(
      "cat .agents/skills/x/SKILL.md"
    );
    expect(unwrapShellCommand(`/bin/bash -lc "printf 'hi' > \\"a b\\""`)).toBe(
      `printf 'hi' > "a b"`
    );
    expect(unwrapShellCommand(`/bin/bash -lc 'echo '\\''x'\\'''`)).toBe(`echo 'x'`);
    expect(unwrapShellCommand(`bash -lc 'wmill sync push'`)).toBe("wmill sync push");
    expect(unwrapShellCommand(`/bin/bash -lc 'rg -g '"'"'!x'"'"' f'`)).toBe(`rg -g '!x' f`);
  });
});

describe("toToolInvocations", () => {
  it("records reading a SKILL.md as a Skill call before the command", () => {
    const tools = toToolInvocations({
      type: "command_execution",
      command: `/bin/bash -lc 'sed -n 1,200p .agents/skills/write-script-bun/SKILL.md && mkdir f'`,
    });
    expect(tools.map((tool) => [tool.tool, tool.input])).toEqual([
      ["Skill", { skill: "write-script-bun" }],
      ["Bash", { command: "sed -n 1,200p .agents/skills/write-script-bun/SKILL.md && mkdir f" }],
    ]);
  });

  it("records a skill read after a mutation earlier in the same command", () => {
    const tools = toToolInvocations({
      type: "command_execution",
      command: `/bin/bash -lc 'printf main > f/a.ts && cat .agents/skills/write-script-bun/SKILL.md'`,
    });
    expect(tools.map((tool) => tool.tool)).toEqual(["Bash", "Skill"]);
  });

  it("maps a patch to Edit so it counts as a mutation", () => {
    expect(
      toToolInvocations({ type: "file_change", changes: [{ path: "a.ts", kind: "add" }] })[0]
    ).toMatchObject({ tool: "Edit", input: { changes: [{ path: "a.ts", kind: "add" }] } });
  });
});

describe("runCodex", () => {
  it("keeps parallel tool calls in the order they started", async () => {
    const dir = await mkdtemp(join(tmpdir(), "fake-codex-"));
    const events = [
      { type: "item.started", item: { id: "a", type: "command_execution", command: "mkdir f" } },
      { type: "item.started", item: { id: "b", type: "command_execution", command: "cat .agents/skills/s/SKILL.md" } },
      { type: "item.completed", item: { id: "b", type: "command_execution", command: "cat .agents/skills/s/SKILL.md" } },
      { type: "item.completed", item: { id: "a", type: "command_execution", command: "mkdir f" } },
      { type: "item.completed", item: { id: "c", type: "agent_message", text: "done" } },
      { type: "turn.completed", usage: { input_tokens: 10, output_tokens: 2 } },
    ];
    const lines = events.map((event) => `printf '%s\\n' '${JSON.stringify(event)}'`).join("\n");
    await writeFile(join(dir, "codex"), `#!/bin/sh\ncat > /dev/null\n${lines}\n`);
    await chmod(join(dir, "codex"), 0o755);
    try {
      const result = await runCodex("p", dir, 12, "m", { PATH: `${dir}:${process.env.PATH}` });
      expect(result.toolsUsed.map((tool) => tool.tool)).toEqual(["Bash", "Skill", "Bash"]);
      expect(result.output).toBe("done");
      expect(result.tokenUsage).toEqual({ prompt: 10, completion: 2, total: 12 });
    } finally {
      await rm(dir, { recursive: true, force: true });
    }
  });
});
