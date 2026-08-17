import { colors } from "@cliffy/ansi/colors";
import { Command } from "@cliffy/command";
import { Select } from "@cliffy/prompt/select";
import * as log from "../../core/log.ts";
import {
  type AgentsMdMigration,
  type ReconcileOutcome,
  WMILL_INIT_AI_AGENTS_SOURCE_ENV,
  WMILL_INIT_AI_CLAUDE_SOURCE_ENV,
  WMILL_INIT_AI_SKILLS_SOURCE_ENV,
  writeAiGuidanceFiles,
} from "../../guidance/writer.ts";

/**
 * Programmatic entry point reused by `wmill init`. The init flow doesn't
 * register the cliffy command itself — it imports and calls this directly so
 * that prompt regeneration is part of every init.
 */
export async function refreshPrompts(opts: {
  yes?: boolean;
}): Promise<void> {
  // Match `core/conf.ts`'s missing-key default (`?? false`) so legacy
  // wmill.yaml files without the key don't drift from how sync renders
  // paths. New projects get `true` via the wmill.yaml template, not via
  // this fallback.
  let nonDottedPaths = false;
  try {
    const { readConfigFile } = await import("../../core/conf.ts");
    const config = await readConfigFile();
    nonDottedPaths = config.nonDottedPaths ?? false;
  } catch {
    // If config can't be read, use the conservative default above.
  }

  const assumeYes = opts.yes === true;
  const interactive = process.stdin.isTTY && !assumeYes;

  try {
    const result = await writeAiGuidanceFiles({
      targetDir: ".",
      nonDottedPaths,
      skillsSourcePath: process.env[WMILL_INIT_AI_SKILLS_SOURCE_ENV],
      agentsSourcePath: process.env[WMILL_INIT_AI_AGENTS_SOURCE_ENV],
      claudeSourcePath: process.env[WMILL_INIT_AI_CLAUDE_SOURCE_ENV],
      resolveAgentsMdMigration: async () => {
        // Consent model (matches `wmill refresh tsconfig`): we only touch an
        // existing user-owned file that we don't recognize when the user opts
        // in. `--yes` (and `wmill init --default`) appends without asking; an
        // interactive run prompts; a plain non-interactive run leaves it alone.
        if (assumeYes) return "append";
        if (interactive) return await promptMigration();
        return "skip";
      },
    });

    log.info(colors.green("Refreshed AGENTS.wmill.md"));

    if (result.legacyManagedRemoved) {
      log.info(
        colors.yellow(
          "Migrated legacy AGENTS.cli.md → AGENTS.wmill.md (removed the old file and rewrote any @AGENTS.cli.md include)."
        )
      );
    }

    reportReconciliation({
      file: "AGENTS.md",
      includeLine: "@AGENTS.wmill.md",
      created: result.agentsCreated,
      migration: result.agentsMigration,
    });

    reportReconciliation({
      file: "CLAUDE.md",
      includeLine: "@AGENTS.md",
      created: result.claudeCreated,
      migration: result.claudeMigration,
    });

    log.info(
      colors.green(
        `Refreshed .claude/skills/ and .agents/skills/ with ${result.skillCount} skills`
      )
    );
    log.info(
      colors.gray(
        "Project-specific instructions live in AGENTS.md (never overwritten unless you opt in)."
      )
    );
  } catch (error) {
    // Log first so the user sees what happened, then rethrow so `wmill
    // refresh prompts` (and `wmill init`, which delegates here) exits
    // non-zero. Silent swallowing would hide a broken refresh from CI.
    if (error instanceof Error) {
      log.error(`Could not refresh guidance files: ${error.message}`);
    } else {
      log.error(`Could not refresh guidance files: ${error}`);
    }
    throw error;
  }
}

function reportReconciliation(opts: {
  file: string;
  includeLine: string;
  created: boolean;
  migration: ReconcileOutcome;
}): void {
  if (opts.created) {
    log.info(colors.green(`Created ${opts.file} (user-owned)`));
    return;
  }
  switch (opts.migration) {
    case "already-linked":
      log.info(
        colors.gray(
          `${opts.file} already references ${opts.includeLine} — left as-is`
        )
      );
      break;
    case "append":
      log.info(
        colors.green(`Appended ${opts.includeLine} include to existing ${opts.file}`)
      );
      break;
    case "overwrite":
      log.info(colors.yellow(`Overwrote ${opts.file} with managed skeleton`));
      break;
    case "skip":
      log.info(
        colors.gray(
          `${opts.file} left unchanged — wire \`${opts.includeLine}\` in manually when ready`
        )
      );
      break;
    case "not-applicable":
      // unreachable when created is false, but keep exhaustive
      break;
  }
}

async function promptMigration(): Promise<AgentsMdMigration> {
  log.info("");
  log.info(
    colors.yellow(
      "An existing AGENTS.md or CLAUDE.md was found that does not reference Windmill's managed guidance."
    )
  );
  log.info(
    colors.gray(
      "Choose how to link the managed files in (we'll apply the same choice to AGENTS.md and CLAUDE.md):"
    )
  );

  const choice = await Select.prompt({
    message: "How should we handle the existing file(s)?",
    options: [
      {
        name:
          "Append the include line " +
          "(preserves your content — recommended if you have custom instructions)",
        value: "append",
      },
      {
        name:
          "Overwrite with the managed skeleton " +
          "(replaces your content — pick if the file only had the default template)",
        value: "overwrite",
      },
      {
        name: "Skip — leave the file alone; I'll wire it up manually",
        value: "skip",
      },
    ],
  });

  return choice as AgentsMdMigration;
}

interface CommandOptions {
  yes?: boolean;
}

async function promptsAction(opts: CommandOptions): Promise<void> {
  await refreshPrompts({ yes: opts.yes === true });
}

const command = new Command()
  .description("Refresh AGENTS.wmill.md and managed skills. User-owned AGENTS.md and CLAUDE.md are never overwritten unless you opt in.")
  .option(
    "--yes",
    "Non-interactive: append the @AGENTS.wmill.md include to an existing AGENTS.md / CLAUDE.md without prompting. Without it, a non-interactive run leaves an unlinked file untouched."
  )
  .action(promptsAction as any);

export default command;
