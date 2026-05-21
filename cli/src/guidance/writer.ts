import { cp, mkdir, readdir, stat, writeFile } from "node:fs/promises";
import { readTextFile } from "../utils/utils.ts";
import { join } from "node:path";
import {
  AGENTS_CLI_INCLUDE_LINE,
  generateAgentsCliMdContent,
  generateAgentsMdSkeleton,
} from "./core.ts";
import {
  currentPromptsHash,
  injectPromptsHashMarker,
} from "./freshness.ts";
import {
  SCHEMAS,
  SCHEMA_MAPPINGS,
  SKILLS,
  SKILL_CONTENT,
  type SkillMetadata,
} from "./skills.gen.ts";

type ResolvedSkillMetadata = SkillMetadata & {
  directoryName: string;
};

/**
 * How to reconcile an existing AGENTS.md that doesn't reference AGENTS.cli.md.
 *
 * - `append`: leave AGENTS.md as-is and append the `@AGENTS.cli.md` include.
 * - `overwrite`: replace AGENTS.md with the managed skeleton.
 * - `skip`: leave AGENTS.md alone. AGENTS.cli.md is still written/refreshed,
 *   but no link from AGENTS.md to it — the user is expected to wire it
 *   manually later.
 */
export type AgentsMdMigration = "append" | "overwrite" | "skip";

export interface WriteAiGuidanceOptions {
  targetDir: string;
  nonDottedPaths?: boolean;
  /** Skill source override (testing / source-of-truth bundling). */
  skillsSourcePath?: string;
  /** AGENTS.cli.md source override (testing). */
  agentsSourcePath?: string;
  /** CLAUDE.md source override (testing). */
  claudeSourcePath?: string;
  /**
   * Optional resolver invoked when an existing AGENTS.md lacks an
   * `@AGENTS.cli.md` reference. Callers are expected to prompt the user; if
   * omitted, the writer defaults to `append` (non-destructive).
   */
  resolveAgentsMdMigration?: () => Promise<AgentsMdMigration>;
}

export interface WriteAiGuidanceResult {
  agentsCliWritten: boolean;
  agentsCreated: boolean;
  agentsMigration: AgentsMdMigration | "already-linked" | "not-applicable";
  claudeWritten: boolean;
  skillCount: number;
}

export const WMILL_INIT_AI_SKILLS_SOURCE_ENV = "WMILL_INIT_AI_SKILLS_SOURCE";
export const WMILL_INIT_AI_AGENTS_SOURCE_ENV = "WMILL_INIT_AI_AGENTS_SOURCE";
export const WMILL_INIT_AI_CLAUDE_SOURCE_ENV = "WMILL_INIT_AI_CLAUDE_SOURCE";

const CLAUDE_MD_DEFAULT = "Instructions are in @AGENTS.md\n";

/**
 * Both `.agents/skills/` (read by Codex, Pi) and `.claude/skills/` (read by
 * Claude Code) receive the full skill content. We can't use `@<path>` to
 * deduplicate because Claude's skill loader reads SKILL.md as-is — it does
 * not expand `@` references inside skill bodies (those work only in
 * AGENTS.md / CLAUDE.md).
 */
const SKILL_TARGET_ROOTS = [".agents", ".claude"] as const;

export async function writeAiGuidanceFiles(
  options: WriteAiGuidanceOptions
): Promise<WriteAiGuidanceResult> {
  // Match `core/conf.ts`'s missing-key default — if a legacy wmill.yaml
  // omits `nonDottedPaths`, sync treats it as `false`, so we must too or
  // the freshness hash will be permanently out of sync with the rest of
  // the CLI's view of the project.
  const nonDottedPaths = options.nonDottedPaths ?? false;
  const skillMetadata = options.skillsSourcePath
    ? await readSkillMetadataFromDirectory(options.skillsSourcePath)
    : getGeneratedSkillMetadata();

  // AGENTS.cli.md — always (re)written, this is the managed file.
  // We embed a content-hash marker so other `wmill` commands can detect a
  // stale bundle and prompt the user to `wmill refresh prompts`.
  const rawAgentsCliContent =
    options.agentsSourcePath != null
      ? await readTextFile(options.agentsSourcePath)
      : generateAgentsCliMdContent(buildSkillsReference(skillMetadata));
  const agentsCliContent = injectPromptsHashMarker(
    rawAgentsCliContent,
    currentPromptsHash(nonDottedPaths)
  );
  const agentsCliPath = join(options.targetDir, "AGENTS.cli.md");
  await writeFile(agentsCliPath, agentsCliContent, "utf8");
  const agentsCliWritten = true;

  // AGENTS.md — user-owned. Three paths:
  //   1. doesn't exist → create skeleton (which already includes @AGENTS.cli.md).
  //   2. exists and already references @AGENTS.cli.md → leave alone.
  //   3. exists but doesn't reference @AGENTS.cli.md → ask caller via
  //      resolveAgentsMdMigration (defaults to append).
  const agentsMdPath = join(options.targetDir, "AGENTS.md");
  const agentsMdResult = await reconcileAgentsMd({
    path: agentsMdPath,
    resolveMigration: options.resolveAgentsMdMigration,
  });

  const claudeWritten = await writeProjectGuidanceFile({
    targetPath: join(options.targetDir, "CLAUDE.md"),
    overwrite: false,
    content:
      options.claudeSourcePath != null
        ? await readTextFile(options.claudeSourcePath)
        : CLAUDE_MD_DEFAULT,
  });

  if (options.skillsSourcePath) {
    await copySkillsFromSource(options.targetDir, options.skillsSourcePath);
  } else {
    await writeGeneratedSkills(options.targetDir, nonDottedPaths);
  }

  return {
    agentsCliWritten,
    agentsCreated: agentsMdResult.created,
    agentsMigration: agentsMdResult.migration,
    claudeWritten,
    skillCount: skillMetadata.length,
  };
}

async function reconcileAgentsMd(options: {
  path: string;
  resolveMigration?: () => Promise<AgentsMdMigration>;
}): Promise<{
  created: boolean;
  migration: AgentsMdMigration | "already-linked" | "not-applicable";
}> {
  const exists = (await stat(options.path).catch(() => null)) != null;
  if (!exists) {
    await writeFile(options.path, generateAgentsMdSkeleton(), "utf8");
    return { created: true, migration: "not-applicable" };
  }

  const existing = await readTextFile(options.path);
  if (referencesAgentsCli(existing)) {
    return { created: false, migration: "already-linked" };
  }

  const choice = options.resolveMigration
    ? await options.resolveMigration()
    : "append";

  if (choice === "skip") {
    return { created: false, migration: "skip" };
  }

  if (choice === "overwrite") {
    await writeFile(options.path, generateAgentsMdSkeleton(), "utf8");
    return { created: false, migration: "overwrite" };
  }

  // append — add the include at the end, leaving existing content untouched.
  const appended = existing.endsWith("\n")
    ? `${existing}\n${AGENTS_CLI_INCLUDE_LINE}\n`
    : `${existing}\n\n${AGENTS_CLI_INCLUDE_LINE}\n`;
  await writeFile(options.path, appended, "utf8");
  return { created: false, migration: "append" };
}

function referencesAgentsCli(content: string): boolean {
  // Split on any whitespace (newlines, spaces, tabs, CR) and check for an
  // exact token match. This avoids regex escaping pitfalls and false matches
  // on look-alike strings (`@AGENTS-cli-md`, `@AGENTS.cli.mdx`, ...).
  for (const token of content.split(/\s+/)) {
    if (token === AGENTS_CLI_INCLUDE_LINE) {
      return true;
    }
  }
  return false;
}

function buildSkillsReference(
  skills: Pick<ResolvedSkillMetadata, "directoryName" | "description">[]
): string {
  return skills
    .map(
      (skill) =>
        `- \`.agents/skills/${skill.directoryName}/SKILL.md\` - ${skill.description}`
    )
    .join("\n");
}

async function copySkillsFromSource(
  targetDir: string,
  skillsSourcePath: string
): Promise<ResolvedSkillMetadata[]> {
  const skillsDirs = await ensureSkillsDirectories(targetDir);
  await Promise.all(
    skillsDirs.map((skillsDir) =>
      copyDirectoryContents(skillsSourcePath, skillsDir)
    )
  );
  return await readSkillMetadataFromDirectory(skillsDirs[0]);
}

async function writeGeneratedSkills(
  targetDir: string,
  nonDottedPaths: boolean
): Promise<ResolvedSkillMetadata[]> {
  const skillsDirs = await ensureSkillsDirectories(targetDir);

  await Promise.all(
    skillsDirs.flatMap((skillsDir) =>
      SKILLS.map(async (skill) => {
        const skillDir = join(skillsDir, skill.name);
        await mkdir(skillDir, { recursive: true });
        await writeFile(
          join(skillDir, "SKILL.md"),
          renderGeneratedSkillContent(skill.name, nonDottedPaths),
          "utf8"
        );
      })
    )
  );

  return SKILLS.map((skill) => ({
    ...skill,
    directoryName: skill.name,
  }));
}

function getGeneratedSkillMetadata(): ResolvedSkillMetadata[] {
  return SKILLS.map((skill) => ({
    ...skill,
    directoryName: skill.name,
  }));
}

async function ensureSkillsDirectories(targetDir: string): Promise<string[]> {
  const skillsDirs = SKILL_TARGET_ROOTS.map((root) =>
    join(targetDir, root, "skills")
  );
  await Promise.all(
    skillsDirs.map((skillsDir) => mkdir(skillsDir, { recursive: true }))
  );
  return skillsDirs;
}

async function copyDirectoryContents(
  sourceDir: string,
  targetDir: string
): Promise<void> {
  const entries = await readdir(sourceDir, { withFileTypes: true });

  await Promise.all(
    entries.map(async (entry) => {
      await cp(join(sourceDir, entry.name), join(targetDir, entry.name), {
        recursive: true,
        force: true,
      });
    })
  );
}

function renderGeneratedSkillContent(
  skillName: string,
  nonDottedPaths: boolean
): string {
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
      return formatSchemaForMarkdown(
        schemaYaml,
        mapping.name,
        mapping.filePattern
      );
    })
    .filter((entry): entry is string => entry !== null);

  if (schemaDocs.length === 0) {
    return skillContent;
  }

  return `${skillContent}\n\n${schemaDocs.join("\n\n")}`;
}

async function readSkillMetadataFromDirectory(
  skillsDir: string
): Promise<ResolvedSkillMetadata[]> {
  const entries = await readdir(skillsDir, { withFileTypes: true });
  const skills: ResolvedSkillMetadata[] = [];

  for (const entry of entries.sort((left, right) =>
    left.name.localeCompare(right.name)
  )) {
    if (!entry.isDirectory()) {
      continue;
    }

    const skillPath = join(skillsDir, entry.name, "SKILL.md");
    if (!(await stat(skillPath).catch(() => null))) {
      continue;
    }

    const content = await readTextFile(skillPath);
    skills.push(parseSkillMetadata(content, entry.name));
  }

  return skills;
}

function parseSkillMetadata(
  content: string,
  fallbackName: string
): ResolvedSkillMetadata {
  const frontMatterMatch = content.match(/^---\s*\n([\s\S]*?)\n---/);
  if (!frontMatterMatch) {
    return {
      name: fallbackName,
      description: `Skill loaded from ${fallbackName}`,
      directoryName: fallbackName,
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

  return { name, description, directoryName: fallbackName };
}

async function writeProjectGuidanceFile(options: {
  targetPath: string;
  content: string;
  overwrite: boolean;
}): Promise<boolean> {
  if (
    !options.overwrite &&
    (await stat(options.targetPath).catch(() => null))
  ) {
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
