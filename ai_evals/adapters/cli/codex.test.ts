import { describe, expect, it } from "bun:test";
import { toToolInvocations, unwrapShellCommand } from "./codex";

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
