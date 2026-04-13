import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { parse } from "yaml";
import type { EvalCase, EvalMode, FlowValidationSpec } from "./types";

const REPO_ROOT = fileURLToPath(new URL("../../", import.meta.url));
const CASES_DIR = path.join(REPO_ROOT, "ai_evals", "cases");

interface RawEvalCase {
  id: string;
  prompt: string;
  initial?: string;
  expected?: string;
  validate?: FlowValidationSpec;
  judgeChecklist?: string[];
}

export function getRepoRoot(): string {
  return REPO_ROOT;
}

export function getAiEvalsRoot(): string {
  return path.join(REPO_ROOT, "ai_evals");
}

export async function loadCases(mode: EvalMode): Promise<EvalCase[]> {
  const filePath = path.join(CASES_DIR, `${mode}.yaml`);
  const raw = await readFile(filePath, "utf8");
  const parsed = parse(raw);

  if (!Array.isArray(parsed)) {
    throw new Error(`Expected ${filePath} to contain a YAML list of cases`);
  }

  return parsed.map((entry) => ({
    id: entry.id,
    prompt: entry.prompt,
    initialPath: resolveFixturePath(entry.initial),
    expectedPath: resolveFixturePath(entry.expected),
    validate: entry.validate,
    judgeChecklist: entry.judgeChecklist,
  }));
}

export async function loadSelectedCases(
  mode: EvalMode,
  selectedIds: string[]
): Promise<EvalCase[]> {
  const allCases = await loadCases(mode);
  if (selectedIds.length === 0) {
    return allCases;
  }

  const caseMap = new Map(allCases.map((entry) => [entry.id, entry]));
  const missing = selectedIds.filter((id) => !caseMap.has(id));
  if (missing.length > 0) {
    throw new Error(
      `Unknown ${mode} case${missing.length === 1 ? "" : "s"}: ${missing.join(", ")}`
    );
  }

  return selectedIds.map((id) => caseMap.get(id)!);
}

function resolveFixturePath(value: string | undefined): string | undefined {
  if (!value) {
    return undefined;
  }
  return path.isAbsolute(value) ? value : path.join(REPO_ROOT, value);
}
