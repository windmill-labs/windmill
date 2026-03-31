import { cp, mkdtemp, mkdir, readFile, rm, writeFile } from "fs/promises";
import { existsSync } from "fs";
import { tmpdir } from "os";
import { dirname, join } from "path";
import { fileURLToPath } from "url";
import { runPromptAndCapture, type TestResult, wasSkillInvoked, getGeneratedSkillsSource } from "./test-utils";

export interface ExpectedFile {
  path: string;
  mustContain?: string[];
  mustNotContain?: string[];
}

export interface CliArtifactEvalCase {
  id: string;
  description?: string;
  prompt: string;
  maxTurns?: number;
  expectedSkill?: string;
  expectedOutputSubstrings?: string[];
  expectedFiles: ExpectedFile[];
}

export interface ArtifactCheck {
  name: string;
  passed: boolean;
  details?: string;
}

export interface FileArtifactResult {
  path: string;
  exists: boolean;
  content?: string;
}

export interface CliArtifactEvalResult {
  workspaceDir: string;
  renderedPrompt: string;
  run: TestResult;
  checks: ArtifactCheck[];
  expectedFiles: FileArtifactResult[];
  passed: boolean;
}

const CASES_FILE = fileURLToPath(
  new URL("../../../ai_evals/cases/cli/script.json", import.meta.url)
);

export async function loadCliArtifactEvalCases(): Promise<CliArtifactEvalCase[]> {
  const raw = await readFile(CASES_FILE, "utf8");
  const parsed = JSON.parse(raw) as CliArtifactEvalCase[];

  if (!Array.isArray(parsed) || parsed.length === 0) {
    throw new Error(`No CLI artifact eval cases found in ${CASES_FILE}`);
  }

  return parsed;
}

export async function runCliArtifactEvalCase(
  evalCase: CliArtifactEvalCase
): Promise<CliArtifactEvalResult> {
  const workspaceDir = await createIsolatedWorkspace(evalCase.id);

  try {
    const renderedPrompt = renderPrompt(evalCase.prompt, workspaceDir);
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
      passed: checks.every((check) => check.passed)
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

async function createIsolatedWorkspace(caseId: string): Promise<string> {
  const workspaceDir = await mkdtemp(join(tmpdir(), `wmill-cli-artifact-${caseId}-`));
  const skillsDir = join(workspaceDir, ".claude", "skills");
  const generatedSkillsSource = getGeneratedSkillsSource();

  await mkdir(dirname(skillsDir), { recursive: true });
  await cp(generatedSkillsSource, skillsDir, { recursive: true });
  await writeFile(join(workspaceDir, "rt.d.ts"), "export namespace RT {}\n", "utf8");

  return workspaceDir;
}

function renderPrompt(prompt: string, workspaceDir: string): string {
  return prompt.replaceAll("{{workspace_root}}", workspaceDir);
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
  run: TestResult,
  fileResults: FileArtifactResult[]
): ArtifactCheck[] {
  const checks: ArtifactCheck[] = [];

  if (evalCase.expectedSkill) {
    checks.push({
      name: `invokes ${evalCase.expectedSkill}`,
      passed: wasSkillInvoked(run, evalCase.expectedSkill),
      details: `skills invoked: ${run.skillsInvoked.join(", ")}`
    });
  }

  for (const expectedOutput of evalCase.expectedOutputSubstrings ?? []) {
    checks.push({
      name: `mentions '${expectedOutput}' in assistant output`,
      passed: run.output.includes(expectedOutput)
    });
  }

  for (const expectedFile of evalCase.expectedFiles) {
    const fileResult = fileResults.find((entry) => entry.path === expectedFile.path);
    const content = fileResult?.content ?? "";

    checks.push({
      name: `creates ${expectedFile.path}`,
      passed: Boolean(fileResult?.exists)
    });

    for (const requiredSnippet of expectedFile.mustContain ?? []) {
      checks.push({
        name: `${expectedFile.path} contains '${requiredSnippet}'`,
        passed: content.includes(requiredSnippet)
      });
    }

    for (const forbiddenSnippet of expectedFile.mustNotContain ?? []) {
      checks.push({
        name: `${expectedFile.path} avoids '${forbiddenSnippet}'`,
        passed: !content.includes(forbiddenSnippet)
      });
    }
  }

  return checks;
}
