import { describe, expect, test } from "bun:test";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { writeAiGuidanceFiles } from "../src/guidance/writer.ts";

const SKILL_TARGET_ROOTS = [".claude", ".agents"] as const;

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

      for (const customSkillPath of customSkillPaths) {
        expect(await readFile(customSkillPath, "utf8")).toBe(customSkillContent);
      }

      for (const generatedSkillPath of generatedSkillPaths) {
        const generatedSkillContent = await readFile(generatedSkillPath, "utf8");
        expect(generatedSkillContent).not.toBe(staleGeneratedContent);
        expect(generatedSkillContent).toContain("name: write-flow");
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
      const existingGeneratedSkillPaths = await Promise.all(
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

      for (const customSkillPath of customSkillPaths) {
        expect(await readFile(customSkillPath, "utf8")).toBe(customSkillContent);
      }
      for (const existingGeneratedSkillPath of existingGeneratedSkillPaths) {
        expect(await readFile(existingGeneratedSkillPath, "utf8")).toBe(sourceSkillContent);
      }
      for (const skillsDir of skillsDirs) {
        expect(await readFile(join(skillsDir, "bundle-only", "SKILL.md"), "utf8")).toBe(
          bundleOnlySkillContent
        );
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
      expect(agentsCli).toContain(".claude/skills/custom-folder/SKILL.md");
      expect(agentsCli).not.toContain(".claude/skills/write-flow/SKILL.md");
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
        ".claude/skills/"
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

describe("writeAiGuidanceFiles — referencesAgentsCli (via reconciliation)", () => {
  test.each([
    ["bare line", "@AGENTS.cli.md"],
    ["between blank lines", "before\n\n@AGENTS.cli.md\n\nafter"],
    ["leading whitespace then include", "  @AGENTS.cli.md\n"],
    ["CRLF line endings", "line one\r\n@AGENTS.cli.md\r\nline three"],
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
