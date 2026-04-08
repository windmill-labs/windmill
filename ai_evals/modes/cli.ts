import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { dirname, join } from "node:path";
import { readFile } from "node:fs/promises";
import { writeAiGuidanceFiles } from "../../cli/src/guidance/writer.ts";
import { getGeneratedSkillsSource, runPromptAndCapture, CLI_BENCHMARK_MODEL, CLI_BENCHMARK_PROVIDER } from "../adapters/cli/runtime";
import { copyDirectory, readDirectoryFiles } from "../core/files";
import { validateCliWorkspace } from "../core/validators";
import type { ModeRunner } from "../core/types";

const IGNORE_WORKSPACE_FILES = new Set([".claude", "AGENTS.md", "CLAUDE.md", "rt.d.ts"]);

interface CliWorkspaceFixture {
  sourceDir: string;
  files: Record<string, string>;
}

interface CliRunActual {
  assistantOutput: string;
  workspaceFiles: Record<string, string>;
}

const CLAUDE_PROJECT_PREAMBLE = [
  "Follow the project instructions from AGENTS.md exactly.",
  "Before creating or modifying any Windmill entity, you MUST invoke the relevant Skill tool and follow it.",
  "Use the skill guidance for file layout, implementation details, and the exact next commands to tell the user.",
  "Do not skip the Skill step.",
].join(" ");

export function createCliModeRunner(): ModeRunner<CliWorkspaceFixture, CliWorkspaceFixture, CliRunActual> {
  return {
    mode: "cli",
    concurrency: 1,
    judgeThreshold: 80,
    async loadInitial(path) {
      return path
        ? {
            sourceDir: path,
            files: await readDirectoryFiles(path),
          }
        : undefined;
    },
    async loadExpected(path) {
      return path
        ? {
            sourceDir: path,
            files: await readDirectoryFiles(path),
          }
        : undefined;
    },
    async run(prompt, initial, _context) {
      const workspaceDir = await mkdtemp(join(tmpdir(), "wmill-cli-benchmark-"));

      try {
        if (initial) {
          await copyDirectory(initial.sourceDir, workspaceDir);
        }
        await mkdir(dirname(join(workspaceDir, ".claude", "skills")), { recursive: true });
        await writeAiGuidanceFiles({
          targetDir: workspaceDir,
          nonDottedPaths: true,
          overwriteProjectGuidance: true,
          skillsSourcePath: getGeneratedSkillsSource(),
        });
        await writeFile(join(workspaceDir, "rt.d.ts"), "export namespace RT {}\n", "utf8");

        const renderedPrompt = await renderPrompt(prompt, workspaceDir);
        const run = await runPromptAndCapture(renderedPrompt, workspaceDir, 6);
        const workspaceFiles = await readDirectoryFiles(workspaceDir, { ignore: IGNORE_WORKSPACE_FILES });

        return {
          success: true,
          actual: {
            assistantOutput: run.output,
            workspaceFiles,
          },
          assistantMessageCount: run.assistantMessageCount,
          toolCallCount: run.toolsUsed.length,
          toolsUsed: run.toolsUsed.map((entry) => entry.tool),
          skillsInvoked: run.skillsInvoked,
        };
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        return {
          success: false,
          actual: {
            assistantOutput: "",
            workspaceFiles: {},
          },
          error: message,
          assistantMessageCount: 0,
          toolCallCount: 0,
          toolsUsed: [],
          skillsInvoked: [],
        };
      } finally {
        await rm(workspaceDir, { recursive: true, force: true });
      }
    },
    validate({ actual, initial, expected }) {
      return validateCliWorkspace({
        actualFiles: actual.workspaceFiles,
        expectedFiles: expected?.files,
        initialFiles: initial?.files,
      });
    },
  };
}

export function getCliRunModelLabel(): string {
  return `${CLI_BENCHMARK_PROVIDER}:${CLI_BENCHMARK_MODEL}`;
}

async function renderPrompt(prompt: string, workspaceDir: string): Promise<string> {
  const renderedUserPrompt = prompt.replaceAll("{{workspace_root}}", workspaceDir);
  const agentsInstructions = await readFile(path.join(workspaceDir, "AGENTS.md"), "utf8");

  return [
    "# Project Instructions",
    agentsInstructions.trim(),
    "",
    "# Benchmark Harness",
    CLAUDE_PROJECT_PREAMBLE,
    "",
    "# User Request",
    renderedUserPrompt,
  ].join("\n");
}
