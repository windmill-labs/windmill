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
 * How to reconcile an existing user-owned guidance file (AGENTS.md or
 * CLAUDE.md) that doesn't reference the managed file below it
 * (`@AGENTS.cli.md` for AGENTS.md, `@AGENTS.md` for CLAUDE.md).
 *
 * - `append`: leave the file as-is and append the include line.
 * - `overwrite`: replace the file with the managed skeleton.
 * - `skip`: leave the file alone. The managed downstream file is still
 *   written/refreshed, but no link to it — the user is expected to wire it
 *   manually later.
 */
export type AgentsMdMigration = "append" | "overwrite" | "skip";

export type ReconcileOutcome =
  | AgentsMdMigration
  | "already-linked"
  | "not-applicable";

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
  agentsMigration: ReconcileOutcome;
  claudeCreated: boolean;
  claudeMigration: ReconcileOutcome;
  skillCount: number;
}

export const WMILL_INIT_AI_SKILLS_SOURCE_ENV = "WMILL_INIT_AI_SKILLS_SOURCE";
export const WMILL_INIT_AI_AGENTS_SOURCE_ENV = "WMILL_INIT_AI_AGENTS_SOURCE";
export const WMILL_INIT_AI_CLAUDE_SOURCE_ENV = "WMILL_INIT_AI_CLAUDE_SOURCE";

const CLAUDE_MD_DEFAULT = "Instructions are in @AGENTS.md\n";
const CLAUDE_MD_INCLUDE_LINE = "@AGENTS.md";

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

  // Cache the user's first migration answer and reuse it for every file
  // that needs reconciling in this run — there's never a good reason to ask
  // the same question twice in a row.
  const resolveMigration = cacheOnce(options.resolveAgentsMdMigration);

  // AGENTS.md — user-owned. Three paths:
  //   1. doesn't exist → create skeleton (which already includes @AGENTS.cli.md).
  //   2. exists and already references @AGENTS.cli.md → leave alone.
  //   3. exists but doesn't reference @AGENTS.cli.md → ask caller via
  //      resolveMigration (defaults to append).
  const agentsMdResult = await reconcileIncludingFile({
    path: join(options.targetDir, "AGENTS.md"),
    includeLine: AGENTS_CLI_INCLUDE_LINE,
    skeleton: generateAgentsMdSkeleton(),
    resolveMigration,
  });

  // CLAUDE.md — user-owned wrapper that points at @AGENTS.md. Same three-way
  // reconciliation: create if missing, leave alone if it already references
  // AGENTS.md, otherwise ask via resolveMigration.
  const claudeSkeleton =
    options.claudeSourcePath != null
      ? await readTextFile(options.claudeSourcePath)
      : CLAUDE_MD_DEFAULT;
  const claudeMdResult = await reconcileIncludingFile({
    path: join(options.targetDir, "CLAUDE.md"),
    includeLine: CLAUDE_MD_INCLUDE_LINE,
    skeleton: claudeSkeleton,
    resolveMigration,
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
    claudeCreated: claudeMdResult.created,
    claudeMigration: claudeMdResult.migration,
    skillCount: skillMetadata.length,
  };
}

function cacheOnce(
  resolver: (() => Promise<AgentsMdMigration>) | undefined
): (() => Promise<AgentsMdMigration>) | undefined {
  if (!resolver) return undefined;
  let cached: AgentsMdMigration | null = null;
  return async () => {
    if (cached !== null) return cached;
    cached = await resolver();
    return cached;
  };
}

async function reconcileIncludingFile(options: {
  path: string;
  includeLine: string;
  skeleton: string;
  resolveMigration?: () => Promise<AgentsMdMigration>;
}): Promise<{ created: boolean; migration: ReconcileOutcome }> {
  const exists = (await stat(options.path).catch(() => null)) != null;
  if (!exists) {
    await writeFile(options.path, options.skeleton, "utf8");
    return { created: true, migration: "not-applicable" };
  }

  const existing = await readTextFile(options.path);
  if (referencesIncludeLine(existing, options.includeLine)) {
    return { created: false, migration: "already-linked" };
  }

  const choice = options.resolveMigration
    ? await options.resolveMigration()
    : "append";

  if (choice === "skip") {
    return { created: false, migration: "skip" };
  }

  if (choice === "overwrite") {
    await writeFile(options.path, options.skeleton, "utf8");
    return { created: false, migration: "overwrite" };
  }

  // append — add the include at the end, leaving existing content untouched.
  const appended = existing.endsWith("\n")
    ? `${existing}\n${options.includeLine}\n`
    : `${existing}\n\n${options.includeLine}\n`;
  await writeFile(options.path, appended, "utf8");
  return { created: false, migration: "append" };
}

function referencesIncludeLine(content: string, includeLine: string): boolean {
  // Match when the include appears as a whitespace-separated token on any
  // line that isn't an HTML comment. We can't require the include to be on a
  // line by itself: our own CLAUDE.md default is `Instructions are in
  // @AGENTS.md` (one sentence), and a strict equality check made `wmill
  // refresh prompts` re-prompt every run on files wmill itself wrote.
  // Skipping comment-bearing lines keeps `<!-- @AGENTS.cli.md -->` from
  // false-positiving.
  for (const line of content.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (trimmed.startsWith("<!--") || trimmed.endsWith("-->")) {
      continue;
    }
    if (trimmed.split(/\s+/).includes(includeLine)) {
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
