import { describe, expect, test } from "bun:test";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { writeAiGuidanceFiles } from "../src/guidance/writer.ts";
import {
  currentPromptsHash,
  extractPromptsHash,
  injectPromptsHashMarker,
  shouldRunFreshnessCheck,
  warnIfPromptsStale,
} from "../src/guidance/freshness.ts";

const SKILL_TARGET_ROOTS = [".agents", ".claude"] as const;

async function withTempDir(fn: (tempDir: string) => Promise<void>): Promise<void> {
  const tempDir = await mkdtemp(join(tmpdir(), "wmill_guidance_writer_"));
  try {
    await fn(tempDir);
  } finally {
    await rm(tempDir, { recursive: true, force: true });
  }
}

async function writeSkill(
  rootDir: string,
  skillName: string,
  content: string
): Promise<string> {
  const skillPath = join(rootDir, skillName, "SKILL.md");
  await mkdir(join(rootDir, skillName), { recursive: true });
  await writeFile(skillPath, content, "utf8");
  return skillPath;
}

describe("writeAiGuidanceFiles — skills", () => {
  test("preserves custom skills when refreshing generated guidance", async () => {
    await withTempDir(async (tempDir) => {
      const skillsDirs = SKILL_TARGET_ROOTS.map((root) =>
        join(tempDir, root, "skills")
      );
      const customSkillContent = `---
name: custom-skill
description: Custom skill
---

Preserve me.
`;
      const staleGeneratedContent = "stale generated skill";

      const customSkillPaths = await Promise.all(
        skillsDirs.map((skillsDir) =>
          writeSkill(skillsDir, "custom-skill", customSkillContent)
        )
      );
      const generatedSkillPaths = await Promise.all(
        skillsDirs.map((skillsDir) =>
          writeSkill(skillsDir, "write-flow", staleGeneratedContent)
        )
      );

      await writeAiGuidanceFiles({ targetDir: tempDir });

      // Custom skills survive on every side, untouched.
      for (const customSkillPath of customSkillPaths) {
        expect(await readFile(customSkillPath, "utf8")).toBe(customSkillContent);
      }

      // Both `.agents/skills/` and `.claude/skills/` hold the same full
      // canonical content. (Claude's skill loader doesn't expand `@`
      // references inside SKILL.md, so we can't dedupe via `@`-include.)
      for (const generatedSkillPath of generatedSkillPaths) {
        const generatedSkillContent = await readFile(generatedSkillPath, "utf8");
        expect(generatedSkillContent).not.toBe(staleGeneratedContent);
        expect(generatedSkillContent).toContain("name: write-flow");
        expect(generatedSkillContent).not.toContain("@../../../");
      }
    });
  });

  test("preserves custom skills when copying a skill bundle from source", async () => {
    await withTempDir(async (tempDir) => {
      const skillsDirs = SKILL_TARGET_ROOTS.map((root) =>
        join(tempDir, root, "skills")
      );
      const customSkillContent = `---
name: custom-skill
description: Custom skill
---

Keep this local skill.
`;
      const sourceSkillContent = `---
name: write-flow
description: Replacement flow skill
---

Copied from source.
`;
      const bundleOnlySkillContent = `---
name: bundle-only
description: Bundle only skill
---

Copied from source bundle.
`;

      const customSkillPaths = await Promise.all(
        skillsDirs.map((skillsDir) =>
          writeSkill(skillsDir, "custom-skill", customSkillContent)
        )
      );
      await Promise.all(
        skillsDirs.map((skillsDir) =>
          writeSkill(skillsDir, "write-flow", "old content")
        )
      );
      const sourceSkillsDir = join(tempDir, "source-skills");

      await writeSkill(sourceSkillsDir, "write-flow", sourceSkillContent);
      await writeSkill(sourceSkillsDir, "bundle-only", bundleOnlySkillContent);

      await writeAiGuidanceFiles({
        targetDir: tempDir,
        skillsSourcePath: sourceSkillsDir,
      });

      // Custom skills survive untouched on every side.
      for (const customSkillPath of customSkillPaths) {
        expect(await readFile(customSkillPath, "utf8")).toBe(customSkillContent);
      }

      // Source bundle is copied verbatim into both `.agents/skills/` and
      // `.claude/skills/`. No `@`-include wrapping.
      for (const root of SKILL_TARGET_ROOTS) {
        expect(
          await readFile(join(tempDir, root, "skills/write-flow/SKILL.md"), "utf8")
        ).toBe(sourceSkillContent);
        expect(
          await readFile(join(tempDir, root, "skills/bundle-only/SKILL.md"), "utf8")
        ).toBe(bundleOnlySkillContent);
      }
    });
  });

  test("AGENTS.cli.md gets the skills reference from copied directory names", async () => {
    await withTempDir(async (tempDir) => {
      const sourceSkillsDir = join(tempDir, "source-skills");
      await writeSkill(
        sourceSkillsDir,
        "custom-folder",
        `---
name: write-flow
description: Custom bundle skill
---

Copied from source bundle.
`
      );

      await writeAiGuidanceFiles({
        targetDir: tempDir,
        skillsSourcePath: sourceSkillsDir,
      });

      const agentsCli = await readFile(join(tempDir, "AGENTS.cli.md"), "utf8");
      expect(agentsCli).toContain(".agents/skills/custom-folder/SKILL.md");
      expect(agentsCli).not.toContain(".agents/skills/write-flow/SKILL.md");
      // The skill reference points at the .agents/ tree — not .claude/ —
      // so the path is meaningful to Codex/Pi as well as Claude.
      expect(agentsCli).not.toContain(".claude/skills/custom-folder/SKILL.md");
    });
  });

  test("AGENTS.cli.md and CLAUDE.md are written even if skills creation fails", async () => {
    await withTempDir(async (tempDir) => {
      // Create a file at .claude so mkdir of .claude/skills throws.
      await writeFile(join(tempDir, ".claude"), "not a directory\n", "utf8");

      await expect(
        writeAiGuidanceFiles({ targetDir: tempDir })
      ).rejects.toThrow();

      expect(await readFile(join(tempDir, "AGENTS.cli.md"), "utf8")).toContain(
        ".agents/skills/"
      );
      expect(await readFile(join(tempDir, "AGENTS.md"), "utf8")).toContain(
        "@AGENTS.cli.md"
      );
      expect(await readFile(join(tempDir, "CLAUDE.md"), "utf8")).toContain(
        "@AGENTS.md"
      );
    });
  });
});

describe("writeAiGuidanceFiles — AGENTS.md reconciliation", () => {
  test("creates a skeleton AGENTS.md (with @AGENTS.cli.md include) when none exists", async () => {
    await withTempDir(async (tempDir) => {
      const result = await writeAiGuidanceFiles({ targetDir: tempDir });
      expect(result.agentsCreated).toBe(true);
      expect(result.agentsMigration).toBe("not-applicable");

      const agentsMd = await readFile(join(tempDir, "AGENTS.md"), "utf8");
      expect(agentsMd).toContain("@AGENTS.cli.md");
    });
  });

  test("leaves an existing AGENTS.md alone when it already references @AGENTS.cli.md", async () => {
    await withTempDir(async (tempDir) => {
      const original = "# My AGENTS.md\n\nlocal stuff\n\n@AGENTS.cli.md\n";
      await writeFile(join(tempDir, "AGENTS.md"), original, "utf8");

      const result = await writeAiGuidanceFiles({ targetDir: tempDir });
      expect(result.agentsCreated).toBe(false);
      expect(result.agentsMigration).toBe("already-linked");

      expect(await readFile(join(tempDir, "AGENTS.md"), "utf8")).toBe(original);
    });
  });

  test("appends @AGENTS.cli.md when the resolver returns 'append'", async () => {
    await withTempDir(async (tempDir) => {
      const original = "# Existing custom AGENTS.md\n\nproject rules here.\n";
      await writeFile(join(tempDir, "AGENTS.md"), original, "utf8");

      const result = await writeAiGuidanceFiles({
        targetDir: tempDir,
        resolveAgentsMdMigration: async () => "append",
      });
      expect(result.agentsCreated).toBe(false);
      expect(result.agentsMigration).toBe("append");

      const updated = await readFile(join(tempDir, "AGENTS.md"), "utf8");
      expect(updated).toStartWith(original);
      expect(updated).toContain("@AGENTS.cli.md");
    });
  });

  test("overwrites AGENTS.md with the managed skeleton when the resolver returns 'overwrite'", async () => {
    await withTempDir(async (tempDir) => {
      const original = "# Some old AGENTS.md to be replaced\n";
      await writeFile(join(tempDir, "AGENTS.md"), original, "utf8");

      const result = await writeAiGuidanceFiles({
        targetDir: tempDir,
        resolveAgentsMdMigration: async () => "overwrite",
      });
      expect(result.agentsCreated).toBe(false);
      expect(result.agentsMigration).toBe("overwrite");

      const updated = await readFile(join(tempDir, "AGENTS.md"), "utf8");
      expect(updated).not.toBe(original);
      expect(updated).toContain("@AGENTS.cli.md");
    });
  });

  test("leaves AGENTS.md untouched when the resolver returns 'skip'", async () => {
    await withTempDir(async (tempDir) => {
      const original = "# Hand-managed AGENTS.md\n";
      await writeFile(join(tempDir, "AGENTS.md"), original, "utf8");

      const result = await writeAiGuidanceFiles({
        targetDir: tempDir,
        resolveAgentsMdMigration: async () => "skip",
      });
      expect(result.agentsCreated).toBe(false);
      expect(result.agentsMigration).toBe("skip");

      expect(await readFile(join(tempDir, "AGENTS.md"), "utf8")).toBe(original);
    });
  });

  test("defaults to 'append' when the resolver is not provided", async () => {
    await withTempDir(async (tempDir) => {
      const original = "# AGENTS.md\n";
      await writeFile(join(tempDir, "AGENTS.md"), original, "utf8");

      const result = await writeAiGuidanceFiles({ targetDir: tempDir });
      expect(result.agentsMigration).toBe("append");

      const updated = await readFile(join(tempDir, "AGENTS.md"), "utf8");
      expect(updated).toStartWith(original);
      expect(updated).toContain("@AGENTS.cli.md");
    });
  });
});

describe("writeAiGuidanceFiles — CLAUDE.md reconciliation", () => {
  test("creates a skeleton CLAUDE.md (with @AGENTS.md include) when none exists", async () => {
    await withTempDir(async (tempDir) => {
      const result = await writeAiGuidanceFiles({ targetDir: tempDir });
      expect(result.claudeCreated).toBe(true);
      expect(result.claudeMigration).toBe("not-applicable");

      const claudeMd = await readFile(join(tempDir, "CLAUDE.md"), "utf8");
      expect(claudeMd).toContain("@AGENTS.md");
    });
  });

  test("leaves an existing CLAUDE.md alone when it already references @AGENTS.md", async () => {
    await withTempDir(async (tempDir) => {
      const original = "# My CLAUDE.md\n\nlocal stuff\n\n@AGENTS.md\n";
      await writeFile(join(tempDir, "CLAUDE.md"), original, "utf8");

      const result = await writeAiGuidanceFiles({ targetDir: tempDir });
      expect(result.claudeCreated).toBe(false);
      expect(result.claudeMigration).toBe("already-linked");

      expect(await readFile(join(tempDir, "CLAUDE.md"), "utf8")).toBe(original);
    });
  });

  test("appends @AGENTS.md when the resolver returns 'append'", async () => {
    await withTempDir(async (tempDir) => {
      const original = "# Existing custom CLAUDE.md\n\nBe helpful.\n";
      await writeFile(join(tempDir, "CLAUDE.md"), original, "utf8");

      const result = await writeAiGuidanceFiles({
        targetDir: tempDir,
        resolveAgentsMdMigration: async () => "append",
      });
      expect(result.claudeCreated).toBe(false);
      expect(result.claudeMigration).toBe("append");

      const updated = await readFile(join(tempDir, "CLAUDE.md"), "utf8");
      expect(updated).toStartWith(original);
      expect(updated).toContain("@AGENTS.md");
    });
  });

  test("overwrites CLAUDE.md with the managed skeleton when the resolver returns 'overwrite'", async () => {
    await withTempDir(async (tempDir) => {
      const original = "# Some old CLAUDE.md\n";
      await writeFile(join(tempDir, "CLAUDE.md"), original, "utf8");

      const result = await writeAiGuidanceFiles({
        targetDir: tempDir,
        resolveAgentsMdMigration: async () => "overwrite",
      });
      expect(result.claudeCreated).toBe(false);
      expect(result.claudeMigration).toBe("overwrite");

      expect(
        await readFile(join(tempDir, "CLAUDE.md"), "utf8")
      ).not.toBe(original);
      expect(await readFile(join(tempDir, "CLAUDE.md"), "utf8")).toContain(
        "@AGENTS.md"
      );
    });
  });

  test("leaves CLAUDE.md untouched when the resolver returns 'skip'", async () => {
    await withTempDir(async (tempDir) => {
      const original = "# Hand-managed CLAUDE.md\n";
      await writeFile(join(tempDir, "CLAUDE.md"), original, "utf8");

      const result = await writeAiGuidanceFiles({
        targetDir: tempDir,
        resolveAgentsMdMigration: async () => "skip",
      });
      expect(result.claudeCreated).toBe(false);
      expect(result.claudeMigration).toBe("skip");

      expect(await readFile(join(tempDir, "CLAUDE.md"), "utf8")).toBe(original);
    });
  });

  test("resolver is invoked at most once even if both files need it", async () => {
    await withTempDir(async (tempDir) => {
      const original = "# old\n";
      await writeFile(join(tempDir, "AGENTS.md"), original, "utf8");
      await writeFile(join(tempDir, "CLAUDE.md"), original, "utf8");

      let resolverCalls = 0;
      await writeAiGuidanceFiles({
        targetDir: tempDir,
        resolveAgentsMdMigration: async () => {
          resolverCalls += 1;
          return "append";
        },
      });
      expect(resolverCalls).toBe(1);
    });
  });
});

describe("writeAiGuidanceFiles — referencesAgentsCli (via reconciliation)", () => {
  test.each([
    ["bare line", "@AGENTS.cli.md"],
    ["between blank lines", "before\n\n@AGENTS.cli.md\n\nafter"],
    ["leading whitespace then include", "  @AGENTS.cli.md\n"],
    ["CRLF line endings", "line one\r\n@AGENTS.cli.md\r\nline three"],
    // Mid-sentence include: this is how our own CLAUDE.md default looks
    // ("Instructions are in @AGENTS.md"). A strict line-equality check made
    // `wmill refresh prompts` re-prompt every run on files wmill wrote.
    ["mid-sentence include", "Instructions are in @AGENTS.cli.md\n"],
    // `>` blockquote prefix doesn't disable Claude's `@`-import expansion,
    // so we treat it as a reference too.
    ["blockquoted include", "> @AGENTS.cli.md"],
  ])("treats %s as a reference (no append)", async (_label, content) => {
    await withTempDir(async (tempDir) => {
      await writeFile(join(tempDir, "AGENTS.md"), content, "utf8");
      const result = await writeAiGuidanceFiles({ targetDir: tempDir });
      expect(result.agentsMigration).toBe("already-linked");
      expect(await readFile(join(tempDir, "AGENTS.md"), "utf8")).toBe(content);
    });
  });

  test.each([
    ["@AGENTS.cli.md.backup", "@AGENTS.cli.md.backup"],
    ["@AGENTS.cli.mdx", "@AGENTS.cli.mdx"],
    ["@AGENTS-cli-md (lookalike)", "@AGENTS-cli-md"],
    ["@AGENTS.cli.md without surrounding whitespace", "foo@AGENTS.cli.md"],
    ["commented-out include", "<!-- @AGENTS.cli.md -->"],
  ])("does not treat %s as a reference (append happens)", async (_label, content) => {
    await withTempDir(async (tempDir) => {
      await writeFile(join(tempDir, "AGENTS.md"), content, "utf8");
      const result = await writeAiGuidanceFiles({
        targetDir: tempDir,
        resolveAgentsMdMigration: async () => "append",
      });
      expect(result.agentsMigration).toBe("append");
    });
  });
});

describe("prompts freshness — hash marker", () => {
  test("AGENTS.cli.md written by writeAiGuidanceFiles carries a hash marker", async () => {
    await withTempDir(async (tempDir) => {
      await writeAiGuidanceFiles({ targetDir: tempDir });
      const agentsCli = await readFile(join(tempDir, "AGENTS.cli.md"), "utf8");
      const hash = extractPromptsHash(agentsCli);
      expect(hash).not.toBeNull();
      expect(hash).toMatch(/^[0-9a-f]{12}$/);
    });
  });

  test("the stored hash matches currentPromptsHash for the same nonDottedPaths", async () => {
    await withTempDir(async (tempDir) => {
      // writeAiGuidanceFiles defaults nonDottedPaths to `false` (matching
      // core/conf.ts's missing-key default).
      await writeAiGuidanceFiles({ targetDir: tempDir });
      const agentsCli = await readFile(join(tempDir, "AGENTS.cli.md"), "utf8");
      expect(extractPromptsHash(agentsCli)).toBe(currentPromptsHash(false));
    });
  });

  test("nonDottedPaths setting changes the hash", () => {
    expect(currentPromptsHash(true)).not.toBe(currentPromptsHash(false));
  });

  test("injectPromptsHashMarker places the marker after the H1 title", () => {
    const input = "# Title\n\nbody line\n";
    const out = injectPromptsHashMarker(input, "abc123def456");
    const lines = out.split("\n");
    expect(lines[0]).toBe("# Title");
    expect(lines[1]).toBe("<!-- wmill-prompts-hash: abc123def456 -->");
    expect(lines[2]).toBe("");
    expect(lines[3]).toBe("body line");
  });

  test("injectPromptsHashMarker prepends when there's no H1", () => {
    const input = "no heading\nrest\n";
    const out = injectPromptsHashMarker(input, "abc123def456");
    expect(out).toStartWith("<!-- wmill-prompts-hash: abc123def456 -->");
  });

  test("extractPromptsHash returns null when no marker is present", () => {
    expect(extractPromptsHash("# Title\n\nno marker here\n")).toBeNull();
    expect(extractPromptsHash("<!-- wmill-prompts-hash: tooshort -->")).toBeNull();
  });
});

describe("prompts freshness — shouldRunFreshnessCheck", () => {
  // Each input matches process.argv shape: [node, script, ...args].
  test.each<[string, string[], boolean]>([
    ["empty argv", ["node", "wmill"], false],
    ["wmill --help", ["node", "wmill", "--help"], false],
    ["wmill -h on a subcommand", ["node", "wmill", "init", "-h"], false],
    ["wmill --version", ["node", "wmill", "--version"], false],
    ["wmill init", ["node", "wmill", "init"], false],
    ["wmill init prompts", ["node", "wmill", "init", "prompts"], false],
    ["wmill refresh prompts", ["node", "wmill", "refresh", "prompts"], false],
    ["wmill completions zsh", ["node", "wmill", "completions", "zsh"], false],
    ["wmill upgrade", ["node", "wmill", "upgrade"], false],
    ["wmill sync push", ["node", "wmill", "sync", "push"], true],
    ["wmill flow run", ["node", "wmill", "flow", "run"], true],
    ["wmill --verbose sync push", ["node", "wmill", "--verbose", "sync", "push"], true],
    // Value-taking global options must skip their value when locating the
    // first subcommand. Otherwise `wmill --workspace prod refresh prompts`
    // would misread `"prod"` as the subcommand and trip the warning during
    // the very command that's meant to fix it.
    ["wmill --workspace prod refresh prompts",
      ["node", "wmill", "--workspace", "prod", "refresh", "prompts"], false],
    ["wmill --token tok sync push",
      ["node", "wmill", "--token", "tok", "sync", "push"], true],
    ["wmill --base-url u --workspace w init",
      ["node", "wmill", "--base-url", "u", "--workspace", "w", "init"], false],
    ["wmill --config-dir /etc/wmill init prompts",
      ["node", "wmill", "--config-dir", "/etc/wmill", "init", "prompts"], false],
  ])("%s → %s", (_label, argv, expected) => {
    expect(shouldRunFreshnessCheck(argv)).toBe(expected);
  });
});

describe("prompts freshness — additional invariants", () => {
  test("currentPromptsHash is deterministic across invocations in the same process", () => {
    const h1 = currentPromptsHash(true);
    const h2 = currentPromptsHash(true);
    const h3 = currentPromptsHash(false);
    const h4 = currentPromptsHash(false);
    expect(h1).toBe(h2);
    expect(h3).toBe(h4);
  });

  test("warnIfPromptsStale writes to stderr (never stdout)", async () => {
    await withTempDir(async (tempDir) => {
      // Write a tampered AGENTS.cli.md so the freshness check trips.
      await writeFile(
        join(tempDir, "AGENTS.cli.md"),
        "# Windmill CLI Agent Instructions\n<!-- wmill-prompts-hash: 000000000000 -->\nbody\n",
        "utf8"
      );

      const stdoutWrites: string[] = [];
      const stderrWrites: string[] = [];
      const originalStdout = process.stdout.write.bind(process.stdout);
      const originalStderr = process.stderr.write.bind(process.stderr);
      // @ts-expect-error — overriding write for the test
      process.stdout.write = (chunk: any) => {
        stdoutWrites.push(String(chunk));
        return true;
      };
      // @ts-expect-error — overriding write for the test
      process.stderr.write = (chunk: any) => {
        stderrWrites.push(String(chunk));
        return true;
      };

      try {
        await warnIfPromptsStale({
          cwd: tempDir,
          nonDottedPaths: false,
          argv: ["node", "wmill", "sync", "push"],
        });
      } finally {
        process.stdout.write = originalStdout;
        process.stderr.write = originalStderr;
      }

      const stderrJoined = stderrWrites.join("");
      const stdoutJoined = stdoutWrites.join("");
      expect(stderrJoined).toContain("out of date");
      expect(stdoutJoined).not.toContain("out of date");
    });
  });
});
