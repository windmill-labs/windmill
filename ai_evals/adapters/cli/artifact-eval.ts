import { existsSync } from "fs";
import { cp, mkdtemp, mkdir, readdir, readFile, rm, writeFile } from "fs/promises";
import { tmpdir } from "os";
import { dirname, join } from "path";
import { fileURLToPath } from "url";
import {
  runPromptAndCapture,
  type PromptRunResult,
  wasSkillInvoked
} from "./runtime";
import type { CliVariant } from "./variants";

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
  required?: boolean;
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
  run: PromptRunResult;
  checks: ArtifactCheck[];
  expectedFiles: FileArtifactResult[];
  passed: boolean;
  variantId: string;
}

const CASES_DIR = fileURLToPath(new URL("../../cases/cli", import.meta.url));

export async function loadCliArtifactEvalCases(): Promise<CliArtifactEvalCase[]> {
  const filenames = (await readdir(CASES_DIR))
    .filter((entry) => entry.endsWith(".json"))
    .sort((left, right) => left.localeCompare(right));

  const cases: CliArtifactEvalCase[] = [];

  for (const filename of filenames) {
    const raw = await readFile(join(CASES_DIR, filename), "utf8");
    const parsed = JSON.parse(raw) as CliArtifactEvalCase[];

    if (!Array.isArray(parsed) || parsed.length === 0) {
      throw new Error(`No CLI artifact eval cases found in ${join(CASES_DIR, filename)}`);
    }

    cases.push(...parsed);
  }

  return cases;
}

export async function runCliArtifactEvalCase(
  evalCase: CliArtifactEvalCase,
  options: {
    variant: CliVariant;
  }
): Promise<CliArtifactEvalResult> {
  const workspaceDir = await createIsolatedWorkspace(evalCase.id, options.variant.skillsSourcePath);

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
      passed: checks.every((check) => check.required === false || check.passed),
      variantId: options.variant.id
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
  skillsSourcePath: string
): Promise<string> {
  const workspaceDir = await mkdtemp(join(tmpdir(), `wmill-cli-artifact-${caseId}-`));
  const skillsDir = join(workspaceDir, ".claude", "skills");

  await mkdir(dirname(skillsDir), { recursive: true });
  await cp(skillsSourcePath, skillsDir, { recursive: true });
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
  run: PromptRunResult,
  fileResults: FileArtifactResult[]
): ArtifactCheck[] {
  const checks: ArtifactCheck[] = [];

  if (evalCase.expectedSkill) {
    checks.push({
      name: `invokes ${evalCase.expectedSkill}`,
      passed: wasSkillInvoked(run, evalCase.expectedSkill),
      required: false,
      details: `skills invoked: ${run.skillsInvoked.join(", ")}`
    });
  }

  for (const expectedOutput of evalCase.expectedOutputSubstrings ?? []) {
    checks.push({
      name: `mentions '${expectedOutput}' in assistant output`,
      passed: run.output.includes(expectedOutput),
      required: false
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
