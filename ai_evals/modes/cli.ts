import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { join } from "node:path";
import { readFile } from "node:fs/promises";
import { writeAiGuidanceFiles } from "../../cli/src/guidance/writer.ts";
import type { CliEvalModelConfig } from "../core/models";
import {
  DEFAULT_CLI_EVAL_MODEL,
  formatCliRunModelLabel,
  getGeneratedSkillsSource,
  runPromptAndCapture,
} from "../adapters/cli/runtime";
import { copyDirectory, readDirectoryFiles } from "../core/files";
import { validateCliWorkspace } from "../core/validators";
import type { BenchmarkArtifactFile, CliTrace, ModeRunner } from "../core/types";

const IGNORE_WORKSPACE_FILES = new Set([
  ".claude",
  "AGENTS.md",
  "CLAUDE.md",
  "rt.d.ts",
  ".wmill-benchmark-bin",
  ".wmill-benchmark-wmill-invocations.log",
]);

interface CliWorkspaceFixture {
  sourceDir: string;
  files: Record<string, string>;
}

interface CliRunActual {
  assistantOutput: string;
  workspaceFiles: Record<string, string>;
  trace: CliTrace;
}

const CLAUDE_PROJECT_PREAMBLE = [
  "Follow the project instructions from AGENTS.md exactly.",
  "Before creating or modifying any Windmill entity, you MUST invoke the relevant Skill tool and follow it.",
  "Use the skill guidance for file layout, implementation details, and the exact next commands to tell the user.",
  "Do not skip the Skill step.",
  "You are running inside an automated benchmark harness, not an interactive user session.",
  "Act autonomously and complete the requested file changes directly in the workspace.",
  "Do not ask for confirmation, do not ask the user to save or create files manually, and do not wait for approval.",
  "Do not respond with a plan when you can make the change directly.",
  "Only describe what was done after you have written the files.",
].join(" ");

export function createCliModeRunner(
  modelConfig: CliEvalModelConfig = DEFAULT_CLI_EVAL_MODEL
): ModeRunner<CliWorkspaceFixture, CliWorkspaceFixture, CliRunActual> {
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
    async run(prompt, initial, context) {
      const workspaceDir = await mkdtemp(join(tmpdir(), "wmill-cli-benchmark-"));

      try {
        if (initial) {
          await copyDirectory(initial.sourceDir, workspaceDir);
        }
        await writeAiGuidanceFiles({
          targetDir: workspaceDir,
          nonDottedPaths: true,
          overwriteProjectGuidance: true,
          skillsSourcePath: getGeneratedSkillsSource(),
        });
        await writeFile(join(workspaceDir, "rt.d.ts"), "export namespace RT {}\n", "utf8");

        const renderedPrompt = await renderPrompt(prompt, workspaceDir);
        const run = await runPromptAndCapture(
          renderedPrompt,
          workspaceDir,
          context.evalCase?.runtime?.maxTurns ?? 6,
          modelConfig
        );
        const workspaceFiles = await readDirectoryFiles(workspaceDir, { ignore: IGNORE_WORKSPACE_FILES });

        return {
          success: true,
          actual: {
            assistantOutput: run.output,
            workspaceFiles,
            trace: run.trace,
          },
          assistantMessageCount: run.trace.assistantMessageCount,
          toolCallCount: run.trace.toolsUsed.length,
          toolsUsed: run.trace.toolsUsed.map((entry) => entry.tool),
          skillsInvoked: run.trace.skillsInvoked,
          tokenUsage: run.tokenUsage ?? null,
        };
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        return {
          success: false,
          actual: {
            assistantOutput: "",
            workspaceFiles: {},
            trace: emptyCliTrace(),
          },
          error: message,
          assistantMessageCount: 0,
          toolCallCount: 0,
          toolsUsed: [],
          skillsInvoked: [],
          tokenUsage: null,
        };
      } finally {
        await rm(workspaceDir, { recursive: true, force: true });
      }
    },
    validate({ evalCase, actual, initial, expected }) {
      return validateCliWorkspace({
        actualFiles: actual.workspaceFiles,
        expectedFiles: expected?.files,
        initialFiles: initial?.files,
        assistantOutput: actual.assistantOutput,
        trace: actual.trace,
        cliExpect: evalCase.cliExpect,
      });
    },
    buildArtifacts(actual): BenchmarkArtifactFile[] {
      const artifacts: BenchmarkArtifactFile[] = [
        {
          path: "assistant-output.txt",
          content: `${actual.assistantOutput}\n`,
        },
        {
          path: "trace.json",
          content: JSON.stringify(actual.trace, null, 2) + "\n",
        },
        {
          path: "wmill-invocations.jsonl",
          content:
            actual.trace.wmillInvocations
              .map((entry) => JSON.stringify(entry))
              .join("\n") + (actual.trace.wmillInvocations.length > 0 ? "\n" : ""),
        },
      ];

      for (const [filePath, content] of Object.entries(actual.workspaceFiles)) {
        artifacts.push({
          path: filePath,
          content,
        });
      }

      return artifacts;
    },
  };
}

function emptyCliTrace(): CliTrace {
  return {
    toolsUsed: [],
    skillsInvoked: [],
    assistantMessageCount: 0,
    bashCommands: [],
    proposedCommands: [],
    executedWmillCommands: [],
    wmillInvocations: [],
    firstMutationToolIndex: null,
  };
}

export function getCliRunModelLabel(
  modelConfig: CliEvalModelConfig = DEFAULT_CLI_EVAL_MODEL
): string {
  return formatCliRunModelLabel(modelConfig);
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
