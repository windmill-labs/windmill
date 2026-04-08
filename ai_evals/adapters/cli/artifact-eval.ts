import { existsSync } from "fs";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "fs/promises";
import { tmpdir } from "os";
import { dirname, join } from "path";
import { writeAiGuidanceFiles } from "../../../cli/src/guidance/writer.ts";
import { getGeneratedSkillsSource } from "./runtime";
import {
  loadEvalCases,
  type CliExpectedFileCheck
} from "../shared/evalCases";
import {
  validateCliArtifact,
  type BenchmarkCheck
} from "../shared/validators";
import {
  runPromptAndCapture,
  type PromptRunResult,
} from "./runtime";

export type ExpectedFile = CliExpectedFileCheck;

export interface CliArtifactEvalCase {
  id: string;
  description?: string;
  prompt: string;
  maxTurns?: number;
  expectedSkill?: string;
  expectedOutputSubstrings?: string[];
  expectedFiles: ExpectedFile[];
}

export type ArtifactCheck = BenchmarkCheck;

export interface FileArtifactResult {
  path: string;
  exists: boolean;
  content?: string;
}

export interface CliArtifactEvalResult {
  workspaceDir: string;
  renderedPrompt: string;
  run: PromptRunResult;
  checks: ArtifactCheck[];
  expectedFiles: FileArtifactResult[];
  passed: boolean;
  guidanceLabel: string;
}

export interface CliGuidanceConfig {
  label: string;
  skillsSourcePath?: string;
  agentsSourcePath?: string;
  claudeSourcePath?: string;
}

const CLAUDE_PROJECT_PREAMBLE = [
  "Follow the project instructions from AGENTS.md exactly.",
  "Before creating or modifying any Windmill entity, you MUST invoke the relevant Skill tool and follow it.",
  "Use the skill guidance for file layout, implementation details, and the exact next commands to tell the user.",
  "Do not skip the Skill step."
].join(" ");

export async function loadCliArtifactEvalCases(): Promise<CliArtifactEvalCase[]> {
  return loadEvalCases("cli").map((entry) => ({
    id: entry.id,
    description: entry.title,
    prompt: entry.userPrompt,
    maxTurns:
      typeof entry.workspaceContext.max_turns === "number"
        ? entry.workspaceContext.max_turns
        : undefined,
    expectedSkill: entry.artifactChecks.expectedSkill,
    expectedOutputSubstrings: entry.artifactChecks.expectedOutputSubstrings,
    expectedFiles: entry.artifactChecks.expectedFiles
  }));
}

export async function runCliArtifactEvalCase(
  evalCase: CliArtifactEvalCase,
  options: {
    guidance: CliGuidanceConfig;
  }
): Promise<CliArtifactEvalResult> {
  const workspaceDir = await createIsolatedWorkspace(evalCase.id, options.guidance);

  try {
    const renderedPrompt = await renderPrompt(evalCase.prompt, workspaceDir);
    const run = await runPromptAndCapture(
      renderedPrompt,
      workspaceDir,
      evalCase.maxTurns ?? 6
    );
    const fileResults = await collectExpectedFiles(workspaceDir, evalCase.expectedFiles);
    const checks = buildChecks(evalCase, run, fileResults);

    return {
      workspaceDir,
      renderedPrompt,
      run,
      checks,
      expectedFiles: fileResults,
      passed: checks.every((check) => check.required === false || check.passed),
      guidanceLabel: options.guidance.label
    };
  } catch (error) {
    if (!shouldKeepWorkspace()) {
      await cleanupWorkspace(workspaceDir);
    }
    throw error;
  }
}

export async function cleanupWorkspace(workspaceDir: string): Promise<void> {
  await rm(workspaceDir, { recursive: true, force: true });
}

export function shouldKeepWorkspace(): boolean {
  return process.env.WMILL_CLI_EVAL_KEEP_WORKSPACE === "1";
}

async function createIsolatedWorkspace(
  caseId: string,
  guidance: CliGuidanceConfig
): Promise<string> {
  const workspaceDir = await mkdtemp(join(tmpdir(), `wmill-cli-artifact-${caseId}-`));
  await mkdir(dirname(join(workspaceDir, ".claude", "skills")), { recursive: true });
  await writeAiGuidanceFiles({
    targetDir: workspaceDir,
    nonDottedPaths: true,
    overwriteProjectGuidance: true,
    skillsSourcePath: guidance.skillsSourcePath ?? getGeneratedSkillsSource(),
    agentsSourcePath: guidance.agentsSourcePath,
    claudeSourcePath: guidance.claudeSourcePath,
  });
  await writeFile(join(workspaceDir, "rt.d.ts"), "export namespace RT {}\n", "utf8");

  return workspaceDir;
}

async function renderPrompt(prompt: string, workspaceDir: string): Promise<string> {
  const renderedUserPrompt = prompt.replaceAll("{{workspace_root}}", workspaceDir);
  const agentsInstructions = await readFile(join(workspaceDir, "AGENTS.md"), "utf8");

  return [
    "# Project Instructions",
    agentsInstructions.trim(),
    "",
    "# Benchmark Harness",
    CLAUDE_PROJECT_PREAMBLE,
    "",
    "# User Request",
    renderedUserPrompt
  ].join("\n");
}

async function collectExpectedFiles(
  workspaceDir: string,
  expectedFiles: ExpectedFile[]
): Promise<FileArtifactResult[]> {
  const results: FileArtifactResult[] = [];

  for (const expectedFile of expectedFiles) {
    const absolutePath = join(workspaceDir, expectedFile.path);
    const exists = existsSync(absolutePath);
    if (!exists) {
      results.push({ path: expectedFile.path, exists: false });
      continue;
    }

    results.push({
      path: expectedFile.path,
      exists: true,
      content: await readFile(absolutePath, "utf8")
    });
  }

  return results;
}

function buildChecks(
  evalCase: CliArtifactEvalCase,
  run: PromptRunResult,
  fileResults: FileArtifactResult[]
): ArtifactCheck[] {
  return validateCliArtifact({
    assistantOutput: run.output,
    skillsInvoked: run.skillsInvoked,
    expectedSkill: evalCase.expectedSkill,
    expectedOutputSubstrings: evalCase.expectedOutputSubstrings,
    expectedFiles: evalCase.expectedFiles,
    fileResults
  });
}
