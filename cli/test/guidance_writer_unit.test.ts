import { describe, expect, test } from "bun:test";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { writeAiGuidanceFiles } from "../src/guidance/writer.ts";

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

describe("writeAiGuidanceFiles", () => {
  test("preserves custom skills when refreshing generated guidance", async () => {
    await withTempDir(async (tempDir) => {
      const skillsDir = join(tempDir, ".claude", "skills");
      const customSkillContent = `---
name: custom-skill
description: Custom skill
---

Preserve me.
`;
      const staleGeneratedContent = "stale generated skill";

      const customSkillPath = await writeSkill(skillsDir, "custom-skill", customSkillContent);
      const generatedSkillPath = await writeSkill(skillsDir, "write-flow", staleGeneratedContent);

      await writeAiGuidanceFiles({
        targetDir: tempDir,
        overwriteProjectGuidance: false,
      });

      expect(await readFile(customSkillPath, "utf8")).toBe(customSkillContent);

      const generatedSkillContent = await readFile(generatedSkillPath, "utf8");
      expect(generatedSkillContent).not.toBe(staleGeneratedContent);
      expect(generatedSkillContent).toContain("name: write-flow");
    });
  });

  test("preserves custom skills when copying a skill bundle from source", async () => {
    await withTempDir(async (tempDir) => {
      const skillsDir = join(tempDir, ".claude", "skills");
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

      const customSkillPath = await writeSkill(skillsDir, "custom-skill", customSkillContent);
      const existingGeneratedSkillPath = await writeSkill(skillsDir, "write-flow", "old content");
      const sourceSkillsDir = join(tempDir, "source-skills");

      await writeSkill(sourceSkillsDir, "write-flow", sourceSkillContent);
      const bundleOnlySkillPath = await writeSkill(
        sourceSkillsDir,
        "bundle-only",
        bundleOnlySkillContent
      );

      await writeAiGuidanceFiles({
        targetDir: tempDir,
        overwriteProjectGuidance: false,
        skillsSourcePath: sourceSkillsDir,
      });

      expect(await readFile(customSkillPath, "utf8")).toBe(customSkillContent);
      expect(await readFile(existingGeneratedSkillPath, "utf8")).toBe(sourceSkillContent);
      expect(await readFile(bundleOnlySkillPath.replace(sourceSkillsDir, skillsDir), "utf8")).toBe(
        bundleOnlySkillContent
      );
    });
  });

  test("builds AGENTS skill references from copied directory names", async () => {
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
        overwriteProjectGuidance: false,
        skillsSourcePath: sourceSkillsDir,
      });

      const agentsMd = await readFile(join(tempDir, "AGENTS.md"), "utf8");
      expect(agentsMd).toContain(".claude/skills/custom-folder/SKILL.md");
      expect(agentsMd).not.toContain(".claude/skills/write-flow/SKILL.md");
    });
  });

  test("writes AGENTS.md and CLAUDE.md even if skills creation fails", async () => {
    await withTempDir(async (tempDir) => {
      await writeFile(join(tempDir, ".claude"), "not a directory\n", "utf8");

      await expect(
        writeAiGuidanceFiles({
          targetDir: tempDir,
          overwriteProjectGuidance: false,
        })
      ).rejects.toThrow();

      expect(await readFile(join(tempDir, "AGENTS.md"), "utf8")).toContain(".claude/skills/");
      expect(await readFile(join(tempDir, "CLAUDE.md"), "utf8")).toContain("@AGENTS.md");
    });
  });
});
