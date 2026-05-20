import { colors } from "@cliffy/ansi/colors";
import { Command } from "@cliffy/command";
import * as log from "../../core/log.ts";
import {
  WMILL_INIT_AI_AGENTS_SOURCE_ENV,
  WMILL_INIT_AI_CLAUDE_SOURCE_ENV,
  WMILL_INIT_AI_SKILLS_SOURCE_ENV,
  writeAiGuidanceFiles,
} from "../../guidance/writer.ts";

export async function regenerateAiGuidance(opts: {
  overwriteProjectGuidance: boolean;
}): Promise<void> {
  let nonDottedPaths = true; // default for new inits
  try {
    const { readConfigFile } = await import("../../core/conf.ts");
    const config = await readConfigFile();
    nonDottedPaths = config.nonDottedPaths ?? true;
  } catch {
    // If config can't be read, use defaults
  }

  try {
    const guidanceResult = await writeAiGuidanceFiles({
      targetDir: ".",
      nonDottedPaths,
      overwriteProjectGuidance: opts.overwriteProjectGuidance,
      skillsSourcePath: process.env[WMILL_INIT_AI_SKILLS_SOURCE_ENV],
      agentsSourcePath: process.env[WMILL_INIT_AI_AGENTS_SOURCE_ENV],
      claudeSourcePath: process.env[WMILL_INIT_AI_CLAUDE_SOURCE_ENV],
    });

    if (guidanceResult.agentsWritten) {
      log.info(
        colors.green(
          opts.overwriteProjectGuidance ? "Refreshed AGENTS.md" : "Created AGENTS.md"
        )
      );
    } else if (!opts.overwriteProjectGuidance) {
      log.info(
        colors.gray(
          "AGENTS.md already exists — skipped (run 'wmill init prompts --force' to overwrite)"
        )
      );
    }
    if (guidanceResult.claudeWritten) {
      log.info(
        colors.green(
          opts.overwriteProjectGuidance ? "Refreshed CLAUDE.md" : "Created CLAUDE.md"
        )
      );
    } else if (!opts.overwriteProjectGuidance) {
      log.info(
        colors.gray(
          "CLAUDE.md already exists — skipped (run 'wmill init prompts --force' to overwrite)"
        )
      );
    }
    log.info(
      colors.green(
        `${opts.overwriteProjectGuidance ? "Refreshed" : "Created"} .claude/skills/ and .agents/skills/ with ${guidanceResult.skillCount} skills`
      )
    );
    log.info(
      colors.gray(
        "Project-specific overrides live in AGENTS.custom.md, .claude/skills/custom/, and .agents/skills/custom/ (never overwritten by init)."
      )
    );
  } catch (error) {
    if (error instanceof Error) {
      log.warn(`Could not create guidance files: ${error.message}`);
    } else {
      log.warn(`Could not create guidance files: ${error}`);
    }
  }
}

interface PromptsOptions {
  force?: boolean;
}

async function promptsAction(opts: PromptsOptions): Promise<void> {
  await regenerateAiGuidance({ overwriteProjectGuidance: opts.force === true });
}

const command = new Command()
  .description(
    "Regenerate managed AI guidance files (AGENTS.md, CLAUDE.md, and skills/) " +
      "without touching wmill.yaml or workspace bindings. Files under " +
      ".claude/skills/custom/ and .agents/skills/custom/, plus AGENTS.custom.md, " +
      "are always preserved."
  )
  .option(
    "--force",
    "Also overwrite AGENTS.md and CLAUDE.md (by default they're left alone if present)"
  )
  .action(promptsAction as any);

export default command;
