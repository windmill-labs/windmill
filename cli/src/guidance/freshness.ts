/**
 * Versioning + freshness check for the managed AGENTS.wmill.md bundle.
 *
 * We embed a short hash of "what this CLI would write" into AGENTS.wmill.md as
 * an HTML comment. On every `wmill` command (with a few exceptions), we read
 * the stored hash and compare against the current CLI's hash. Mismatch =>
 * one-line warning telling the user to `wmill refresh prompts`.
 *
 * The hash covers all inputs that affect the rendered bundle: the
 * AGENTS.wmill.md template, every skill body, schemas and schema mappings, and
 * the nonDottedPaths setting. It is *not* tied to the CLI's package version,
 * so non-prompt CLI releases don't produce false positives.
 */
import { createHash } from "node:crypto";
import { stat } from "node:fs/promises";
import { colors } from "@cliffy/ansi/colors";
import { readTextFile } from "../utils/utils.ts";
import {
  AGENTS_WMILL_FILENAME,
  LEGACY_AGENTS_CLI_FILENAME,
  generateAgentsCliMdContent,
} from "./core.ts";
import {
  SCHEMAS,
  SCHEMA_MAPPINGS,
  SKILLS,
  SKILL_CONTENT,
} from "./skills.gen.ts";

// Re-export from the gate module so existing callers (and tests) keep working.
// `shouldRunFreshnessCheck` lives there to avoid pulling skills.gen.ts (~360 KB)
// into main.ts's static import graph; main.ts now imports the gate directly
// and only `await import`s this file lazily.
import { shouldRunFreshnessCheck } from "./freshness_gate.ts";
export { shouldRunFreshnessCheck };

export const PROMPTS_HASH_MARKER_PREFIX = "<!-- wmill-prompts-hash: ";
const PROMPTS_HASH_REGEX = /<!-- wmill-prompts-hash: ([0-9a-f]{12}) -->/;

export function buildPromptsHashMarker(hash: string): string {
  return `${PROMPTS_HASH_MARKER_PREFIX}${hash} -->`;
}

export function extractPromptsHash(content: string): string | null {
  const match = content.match(PROMPTS_HASH_REGEX);
  return match ? match[1] : null;
}

/**
 * Insert the hash marker into rendered AGENTS.wmill.md content. The marker
 * goes on the line right after the title so it's easy to find and doesn't
 * break the rendered Markdown structure.
 */
export function injectPromptsHashMarker(
  content: string,
  hash: string
): string {
  const lines = content.split("\n");
  const marker = buildPromptsHashMarker(hash);
  // Insert right after the first line if it's an H1 title; otherwise
  // prepend so the marker is always near the top.
  const insertAt = lines[0].startsWith("# ") ? 1 : 0;
  lines.splice(insertAt, 0, marker);
  return lines.join("\n");
}

/**
 * Compute the hash for the rendered bundle. The hash is deterministic for a
 * given (CLI bundle, nonDottedPaths) pair.
 */
export function currentPromptsHash(nonDottedPaths: boolean): string {
  const hasher = createHash("sha256");

  // Template structure (without the skills reference — that's hashed
  // separately from the SKILLS metadata).
  hasher.update("template:");
  hasher.update(generateAgentsCliMdContent("__PLACEHOLDER__"));

  // Skill metadata (names + descriptions) — fed into the skills reference
  // line in AGENTS.wmill.md and the wrapper frontmatter.
  hasher.update("\nskills:");
  hasher.update(JSON.stringify(SKILLS));

  // Skill bodies — what actually lands in .agents/skills/<name>/SKILL.md.
  // Sort entries for stable ordering.
  hasher.update("\nbodies:");
  for (const [name, content] of Object.entries(SKILL_CONTENT).sort()) {
    hasher.update("\n");
    hasher.update(name);
    hasher.update("\n");
    hasher.update(content);
  }

  // Schemas + their mappings — embedded inside specific skills.
  hasher.update("\nschemas:");
  hasher.update(JSON.stringify(SCHEMAS));
  hasher.update("\nmappings:");
  hasher.update(JSON.stringify(SCHEMA_MAPPINGS));

  // Path-style setting — controls __flow vs .flow rendering in skill bodies.
  hasher.update("\nnonDotted:");
  hasher.update(String(nonDottedPaths));

  return hasher.digest("hex").slice(0, 12);
}

/**
 * Read the managed guidance file in the current working directory, compare its
 * embedded hash to the current CLI's hash, and print a one-line warning if they
 * differ. Silent on every other code path (no managed file, no marker,
 * matching hash, IO error, …) so it never gets in the user's way.
 *
 * Back-compat: prefers `AGENTS.wmill.md` but falls back to the legacy
 * `AGENTS.cli.md` and runs the exact same staleness check on it. We do NOT
 * warn merely because the old filename is in use — an up-to-date `AGENTS.cli.md`
 * stays quiet; only a stale hash (which `wmill refresh prompts` fixes, and
 * which also migrates the filename) trips the warning.
 */
export async function warnIfPromptsStale(opts?: {
  cwd?: string;
  nonDottedPaths?: boolean;
  argv?: readonly string[];
}): Promise<void> {
  if (opts?.argv && !shouldRunFreshnessCheck(opts.argv)) return;

  const cwd = opts?.cwd ?? process.cwd();

  // Prefer the current filename; fall back to the legacy one for back-compat.
  let fileName = AGENTS_WMILL_FILENAME;
  let path = `${cwd}/${fileName}`;
  if (!(await stat(path).catch(() => null))) {
    fileName = LEGACY_AGENTS_CLI_FILENAME;
    path = `${cwd}/${fileName}`;
    if (!(await stat(path).catch(() => null))) {
      // Neither file present — this project just isn't wmill-managed.
      return;
    }
  }

  let content: string;
  try {
    content = await readTextFile(path);
  } catch {
    return;
  }

  const stored = extractPromptsHash(content);
  if (!stored) {
    // Managed file without a marker. Warn so the user re-runs refresh and
    // picks up the new format (and the new filename).
    emitWarning(
      `Your ${fileName} predates prompt versioning. Run \`wmill refresh prompts\` to refresh and add a version marker.`
    );
    return;
  }

  let nonDottedPaths = opts?.nonDottedPaths;
  if (nonDottedPaths === undefined) {
    try {
      const { readConfigFile } = await import("../core/conf.ts");
      const config = await readConfigFile();
      // Match `core/conf.ts`'s missing-key default (`?? false`); otherwise
      // legacy wmill.yaml files without the key trip a permanent freshness
      // warning even though the prompts are objectively up to date.
      nonDottedPaths = config.nonDottedPaths ?? false;
    } catch {
      nonDottedPaths = false;
    }
  }

  const current = currentPromptsHash(nonDottedPaths);
  if (stored !== current) {
    emitWarning(
      `Your ${fileName} is out of date. Run \`wmill refresh prompts\` to refresh.`
    );
  }
}

/**
 * Send the freshness warning to **stderr** so it never contaminates a
 * downstream pipe (e.g. `wmill job result <id> | jq`). The rest of the CLI
 * uses `log.warn` which writes to stdout — that's wrong for an always-on
 * notification like this one, but we don't want to fix `log.warn` globally
 * in this PR.
 */
function emitWarning(message: string): void {
  process.stderr.write(`${colors.yellow(message)}\n`);
}
