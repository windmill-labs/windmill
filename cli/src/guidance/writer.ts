import { cp, mkdir, readdir, readFile, rm, stat, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { generateAgentsMdContent } from "./core.ts";
import {
  SCHEMAS,
  SCHEMA_MAPPINGS,
  SKILLS,
  SKILL_CONTENT,
  type SkillMetadata,
} from "./skills.ts";

export interface WriteAiGuidanceOptions {
  targetDir: string;
  nonDottedPaths?: boolean;
  overwriteProjectGuidance?: boolean;
  skillsSourcePath?: string;
  agentsSourcePath?: string;
  claudeSourcePath?: string;
}

export interface WriteAiGuidanceResult {
  agentsWritten: boolean;
  claudeWritten: boolean;
  skillsWritten: boolean;
  skillCount: number;
}

export const WMILL_INIT_AI_SKILLS_SOURCE_ENV = "WMILL_INIT_AI_SKILLS_SOURCE";
export const WMILL_INIT_AI_AGENTS_SOURCE_ENV = "WMILL_INIT_AI_AGENTS_SOURCE";
export const WMILL_INIT_AI_CLAUDE_SOURCE_ENV = "WMILL_INIT_AI_CLAUDE_SOURCE";

const CLAUDE_MD_DEFAULT = "Instructions are in @AGENTS.md\n";

export async function writeAiGuidanceFiles(
  options: WriteAiGuidanceOptions
): Promise<WriteAiGuidanceResult> {
  const nonDottedPaths = options.nonDottedPaths ?? true;
  const skillMetadata = options.skillsSourcePath
    ? await copySkillsFromSource(options.targetDir, options.skillsSourcePath)
    : await writeGeneratedSkills(options.targetDir, nonDottedPaths);

  const agentsWritten = await writeProjectGuidanceFile({
    targetPath: join(options.targetDir, "AGENTS.md"),
    overwrite: options.overwriteProjectGuidance ?? false,
    content:
      options.agentsSourcePath != null
        ? await readFile(options.agentsSourcePath, "utf8")
        : generateAgentsMdContent(buildSkillsReference(skillMetadata)),
  });

  const claudeWritten = await writeProjectGuidanceFile({
    targetPath: join(options.targetDir, "CLAUDE.md"),
    overwrite: options.overwriteProjectGuidance ?? false,
    content:
      options.claudeSourcePath != null
        ? await readFile(options.claudeSourcePath, "utf8")
        : CLAUDE_MD_DEFAULT,
  });

  return {
    agentsWritten,
    claudeWritten,
    skillsWritten: true,
    skillCount: skillMetadata.length,
  };
}

function buildSkillsReference(skills: Pick<SkillMetadata, "name" | "description">[]): string {
  return skills
    .map((skill) => `- \`.claude/skills/${skill.name}/SKILL.md\` - ${skill.description}`)
    .join("\n");
}

async function copySkillsFromSource(
  targetDir: string,
  skillsSourcePath: string
): Promise<SkillMetadata[]> {
  const skillsDir = join(targetDir, ".claude", "skills");
  await mkdir(join(targetDir, ".claude"), { recursive: true });
  await rm(skillsDir, { recursive: true, force: true });
  await cp(skillsSourcePath, skillsDir, { recursive: true, force: true });
  return await readSkillMetadataFromDirectory(skillsDir);
}

async function writeGeneratedSkills(
  targetDir: string,
  nonDottedPaths: boolean
): Promise<SkillMetadata[]> {
  const skillsDir = join(targetDir, ".claude", "skills");
  await rm(skillsDir, { recursive: true, force: true });
  await mkdir(skillsDir, { recursive: true });

  await Promise.all(
    SKILLS.map(async (skill) => {
      const skillDir = join(skillsDir, skill.name);
      await mkdir(skillDir, { recursive: true });
      await writeFile(
        join(skillDir, "SKILL.md"),
        renderGeneratedSkillContent(skill.name, nonDottedPaths),
        "utf8"
      );
    })
  );

  return SKILLS;
}

function renderGeneratedSkillContent(skillName: string, nonDottedPaths: boolean): string {
  let skillContent = SKILL_CONTENT[skillName];
  if (!skillContent) {
    throw new Error(`Missing generated skill content for ${skillName}`);
  }

  if (nonDottedPaths) {
    skillContent = skillContent
      .replaceAll("{{FLOW_SUFFIX}}", "__flow")
      .replaceAll("{{APP_SUFFIX}}", "__app")
      .replaceAll("{{RAW_APP_SUFFIX}}", "__raw_app")
      .replaceAll(
        "{{INLINE_SCRIPT_NAMING}}",
        "Inline script files should NOT include `.inline_script.` in their names (e.g. use `a.ts`, not `a.inline_script.ts`)."
      );
  } else {
    skillContent = skillContent
      .replaceAll("{{FLOW_SUFFIX}}", ".flow")
      .replaceAll("{{APP_SUFFIX}}", ".app")
      .replaceAll("{{RAW_APP_SUFFIX}}", ".raw_app")
      .replaceAll(
        "{{INLINE_SCRIPT_NAMING}}",
        "Inline script files use the `.inline_script.` naming convention (e.g. `a.inline_script.ts`)."
      );
  }

  const schemaMappings = SCHEMA_MAPPINGS[skillName];
  if (!schemaMappings || schemaMappings.length === 0) {
    return skillContent;
  }

  const schemaDocs = schemaMappings
    .map((mapping) => {
      const schemaYaml = SCHEMAS[mapping.schemaKey];
      if (!schemaYaml) {
        return null;
      }
      return formatSchemaForMarkdown(schemaYaml, mapping.name, mapping.filePattern);
    })
    .filter((entry): entry is string => entry !== null);

  if (schemaDocs.length === 0) {
    return skillContent;
  }

  return `${skillContent}\n\n${schemaDocs.join("\n\n")}`;
}

async function readSkillMetadataFromDirectory(skillsDir: string): Promise<SkillMetadata[]> {
  const entries = await readdir(skillsDir, { withFileTypes: true });
  const skills: SkillMetadata[] = [];

  for (const entry of entries.sort((left, right) => left.name.localeCompare(right.name))) {
    if (!entry.isDirectory()) {
      continue;
    }

    const skillPath = join(skillsDir, entry.name, "SKILL.md");
    if (!(await stat(skillPath).catch(() => null))) {
      continue;
    }

    const content = await readFile(skillPath, "utf8");
    skills.push(parseSkillMetadata(content, entry.name));
  }

  return skills;
}

function parseSkillMetadata(content: string, fallbackName: string): SkillMetadata {
  const frontMatterMatch = content.match(/^---\s*\n([\s\S]*?)\n---/);
  if (!frontMatterMatch) {
    return {
      name: fallbackName,
      description: `Skill loaded from ${fallbackName}`,
    };
  }

  let name = fallbackName;
  let description = `Skill loaded from ${fallbackName}`;

  for (const line of frontMatterMatch[1].split("\n")) {
    const separatorIndex = line.indexOf(":");
    if (separatorIndex === -1) {
      continue;
    }

    const key = line.slice(0, separatorIndex).trim();
    const value = line.slice(separatorIndex + 1).trim();

    if (key === "name" && value) {
      name = value;
    } else if (key === "description" && value) {
      description = value;
    }
  }

  return { name, description };
}

async function writeProjectGuidanceFile(options: {
  targetPath: string;
  content: string;
  overwrite: boolean;
}): Promise<boolean> {
  if (!options.overwrite && (await stat(options.targetPath).catch(() => null))) {
    return false;
  }

  await writeFile(options.targetPath, options.content, "utf8");
  return true;
}

function formatSchemaForMarkdown(
  schemaYaml: string,
  schemaName: string,
  filePattern: string
): string {
  return `## ${schemaName} (\`${filePattern}\`)

Must be a YAML file that adheres to the following schema:

\`\`\`yaml
${schemaYaml.trim()}
\`\`\``;
}
