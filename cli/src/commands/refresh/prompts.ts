import { colors } from "@cliffy/ansi/colors";
import { Command } from "@cliffy/command";
import { Select } from "@cliffy/prompt/select";
import * as log from "../../core/log.ts";
import {
  type AgentsMdMigration,
  WMILL_INIT_AI_AGENTS_SOURCE_ENV,
  WMILL_INIT_AI_CLAUDE_SOURCE_ENV,
  WMILL_INIT_AI_SKILLS_SOURCE_ENV,
  writeAiGuidanceFiles,
} from "../../guidance/writer.ts";

interface RefreshPromptsOptions {
  yes?: boolean;
  agentsMd?: string;
}

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

  const interactive = process.stdin.isTTY && !opts.yes;

  try {
    const result = await writeAiGuidanceFiles({
      targetDir: ".",
      nonDottedPaths,
      skillsSourcePath: process.env[WMILL_INIT_AI_SKILLS_SOURCE_ENV],
      agentsSourcePath: process.env[WMILL_INIT_AI_AGENTS_SOURCE_ENV],
      claudeSourcePath: process.env[WMILL_INIT_AI_CLAUDE_SOURCE_ENV],
      resolveAgentsMdMigration: async () => {
        if (!interactive) return "append";
        return await promptAgentsMdMigration();
      },
    });

    log.info(colors.green("Refreshed AGENTS.cli.md"));

    if (result.agentsCreated) {
      log.info(colors.green("Created AGENTS.md (user-owned)"));
    } else {
      switch (result.agentsMigration) {
        case "already-linked":
          log.info(
            colors.gray(
              "AGENTS.md already references @AGENTS.cli.md — left as-is"
            )
          );
          break;
        case "append":
          log.info(
            colors.green(
              "Appended @AGENTS.cli.md include to existing AGENTS.md"
            )
          );
          break;
        case "overwrite":
          log.info(
            colors.yellow("Overwrote AGENTS.md with managed skeleton")
          );
          break;
        case "skip":
          log.info(
            colors.gray(
              "AGENTS.md left unchanged — wire `@AGENTS.cli.md` in manually when ready"
            )
          );
          break;
      }
    }

    if (result.claudeWritten) {
      log.info(colors.green("Created CLAUDE.md"));
    }

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
    if (error instanceof Error) {
      log.warn(`Could not refresh guidance files: ${error.message}`);
    } else {
      log.warn(`Could not refresh guidance files: ${error}`);
    }
  }
}

async function promptAgentsMdMigration(): Promise<AgentsMdMigration> {
  log.info("");
  log.info(
    colors.yellow(
      "An existing AGENTS.md was found that does not reference @AGENTS.cli.md."
    )
  );
  log.info(
    colors.gray(
      "Choose how to link Windmill's managed CLI guidance (AGENTS.cli.md) into it:"
    )
  );

  const choice = await Select.prompt({
    message: "How should we handle AGENTS.md?",
    options: [
      {
        name:
          "Append `@AGENTS.cli.md` to your existing AGENTS.md " +
          "(preserves your content — recommended if AGENTS.md has custom instructions)",
        value: "append",
      },
      {
        name:
          "Overwrite AGENTS.md with the managed skeleton " +
          "(replaces your content — pick if AGENTS.md only had the old generated template)",
        value: "overwrite",
      },
      {
        name: "Skip — leave AGENTS.md alone; I'll wire it up manually",
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
  .description("Refresh AGENTS.cli.md, CLAUDE.md, and managed skills. User-owned AGENTS.md is never overwritten unless you opt in.")
  .option(
    "--yes",
    "Non-interactive: skip the migration prompt for existing AGENTS.md without an @AGENTS.cli.md reference; defaults to appending the include."
  )
  .action(promptsAction as any);

export default command;
