import { readdirSync, readFileSync } from "node:fs";
import { join, resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

export type EvalSurfaceName =
  | "cli"
  | "frontend-flow"
  | "frontend-app"
  | "frontend-script";

export interface EvalCaseSummary {
  id: string;
  surface: EvalSurfaceName;
  title: string;
  tags: string[];
}

export interface EvalJudgeRubric {
  minScore?: number;
}

export interface CliExpectedFileCheck {
  path: string;
  mustContain?: string[];
  mustNotContain?: string[];
}

interface RawCliExpectedFileCheck {
  path: string;
  must_contain?: string[];
  must_not_contain?: string[];
}

export interface EvalScriptFixture {
  code: string;
  lang: string;
  path: string;
  args?: Record<string, unknown>;
}

interface ResolvedEvalCaseBase {
  id: string;
  surface: EvalSurfaceName;
  title: string;
  userPrompt: string;
  workspaceContext: Record<string, unknown>;
  judgeRubric: EvalJudgeRubric;
  tags: string[];
}

export interface ResolvedCliEvalCase extends ResolvedEvalCaseBase {
  surface: "cli";
  initialState: Record<string, never>;
  artifactChecks: {
    expectedSkill?: string;
    expectedOutputSubstrings: string[];
    expectedFiles: CliExpectedFileCheck[];
  };
}

export interface ResolvedFrontendFlowEvalCase extends ResolvedEvalCaseBase {
  surface: "frontend-flow";
  initialState: {
    initialFlow?: Record<string, unknown>;
  };
  artifactChecks: {
    expectedFlow: Record<string, unknown>;
  };
}

export interface ResolvedFrontendAppEvalCase extends ResolvedEvalCaseBase {
  surface: "frontend-app";
  initialState: {
    initialAppFixturePath?: string;
  };
  artifactChecks: Record<string, never>;
}

export interface ResolvedFrontendScriptEvalCase extends ResolvedEvalCaseBase {
  surface: "frontend-script";
  initialState: {
    initialScript?: EvalScriptFixture;
  };
  artifactChecks: {
    expectedScript: EvalScriptFixture;
  };
}

export type ResolvedEvalCaseBySurface = {
  cli: ResolvedCliEvalCase;
  "frontend-flow": ResolvedFrontendFlowEvalCase;
  "frontend-app": ResolvedFrontendAppEvalCase;
  "frontend-script": ResolvedFrontendScriptEvalCase;
};

type RawJudgeRubric = {
  min_score?: number;
};

type RawSharedEvalCase = {
  id: string;
  surface: EvalSurfaceName;
  title: string;
  user_prompt: string;
  initial_state: Record<string, unknown>;
  workspace_context: Record<string, unknown>;
  artifact_checks: Record<string, unknown>;
  judge_rubric: RawJudgeRubric;
  tags: string[];
};

type RawCliEvalCase = RawSharedEvalCase & {
  surface: "cli";
  artifact_checks: {
    expected_skill?: string;
    expected_output_substrings?: string[];
    expected_files?: RawCliExpectedFileCheck[];
  };
};

type RawFrontendFlowEvalCase = RawSharedEvalCase & {
  surface: "frontend-flow";
  initial_state: {
    flow_path?: string;
  };
  artifact_checks: {
    expected_flow_path: string;
  };
};

type RawFrontendAppEvalCase = RawSharedEvalCase & {
  surface: "frontend-app";
  initial_state: {
    app_fixture_path?: string;
  };
  artifact_checks: Record<string, never>;
};

type RawFrontendScriptEvalCase = RawSharedEvalCase & {
  surface: "frontend-script";
  initial_state: {
    script_path?: string;
  };
  artifact_checks: {
    expected_script_path: string;
  };
};

type RawEvalCaseBySurface = {
  cli: RawCliEvalCase;
  "frontend-flow": RawFrontendFlowEvalCase;
  "frontend-app": RawFrontendAppEvalCase;
  "frontend-script": RawFrontendScriptEvalCase;
};

const REPO_ROOT = resolve(
  dirname(fileURLToPath(import.meta.url)),
  "../../.."
);

export function loadEvalCaseSummaries(surface: EvalSurfaceName): EvalCaseSummary[] {
  return loadRawEvalCases(surface).map((entry) => ({
    id: entry.id,
    surface: entry.surface,
    title: entry.title,
    tags: [...(entry.tags ?? [])]
  }));
}

export function loadEvalCases<T extends EvalSurfaceName>(
  surface: T
): Array<ResolvedEvalCaseBySurface[T]> {
  return loadRawEvalCases(surface).map((entry) =>
    resolveEvalCase(entry)
  ) as Array<ResolvedEvalCaseBySurface[T]>;
}

function loadRawEvalCases<T extends EvalSurfaceName>(
  surface: T
): Array<RawEvalCaseBySurface[T]> {
  const manifestPaths = getManifestPaths(surface);
  const cases: Array<RawEvalCaseBySurface[T]> = [];

  for (const manifestPath of manifestPaths) {
    const parsed = JSON.parse(readFileSync(manifestPath, "utf8")) as Array<
      RawEvalCaseBySurface[T]
    >;

    if (!Array.isArray(parsed) || parsed.length === 0) {
      throw new Error(`No eval cases found in ${manifestPath}`);
    }

    for (const entry of parsed) {
      if (entry.surface !== surface) {
        throw new Error(
          `Eval case ${entry.id} in ${manifestPath} declared surface ${entry.surface}, expected ${surface}`
        );
      }
      cases.push(entry);
    }
  }

  return cases;
}

function resolveEvalCase(
  entry:
    | RawCliEvalCase
    | RawFrontendFlowEvalCase
    | RawFrontendAppEvalCase
    | RawFrontendScriptEvalCase
):
  | ResolvedCliEvalCase
  | ResolvedFrontendFlowEvalCase
  | ResolvedFrontendAppEvalCase
  | ResolvedFrontendScriptEvalCase {
  const base = {
    id: entry.id,
    surface: entry.surface,
    title: entry.title,
    userPrompt: entry.user_prompt,
    workspaceContext: entry.workspace_context ?? {},
    judgeRubric: normalizeJudgeRubric(entry.judge_rubric),
    tags: [...(entry.tags ?? [])]
  };

  switch (entry.surface) {
    case "cli":
      return {
        ...base,
        surface: "cli",
        initialState: {},
        artifactChecks: {
          expectedSkill: entry.artifact_checks.expected_skill,
          expectedOutputSubstrings:
            entry.artifact_checks.expected_output_substrings ?? [],
          expectedFiles: (entry.artifact_checks.expected_files ?? []).map(
            (file) => ({
              path: file.path,
              mustContain: file.must_contain,
              mustNotContain: file.must_not_contain
            })
          )
        }
      };
    case "frontend-flow":
      return {
        ...base,
        surface: "frontend-flow",
        initialState: {
          initialFlow: entry.initial_state.flow_path
            ? readRepoRelativeJson<Record<string, unknown>>(
                entry.initial_state.flow_path
              )
            : undefined
        },
        artifactChecks: {
          expectedFlow: readRepoRelativeJson<Record<string, unknown>>(
            entry.artifact_checks.expected_flow_path
          )
        }
      };
    case "frontend-app":
      return {
        ...base,
        surface: "frontend-app",
        initialState: {
          initialAppFixturePath: entry.initial_state.app_fixture_path
            ? resolveRepoRelativePath(entry.initial_state.app_fixture_path)
            : undefined
        },
        artifactChecks: {}
      };
    case "frontend-script":
      return {
        ...base,
        surface: "frontend-script",
        initialState: {
          initialScript: entry.initial_state.script_path
            ? readRepoRelativeJson<EvalScriptFixture>(
                entry.initial_state.script_path
              )
            : undefined
        },
        artifactChecks: {
          expectedScript: readRepoRelativeJson<EvalScriptFixture>(
            entry.artifact_checks.expected_script_path
          )
        }
      };
    default:
      return assertNever(entry);
  }
}

function getManifestPaths(surface: EvalSurfaceName): string[] {
  if (surface === "cli") {
    const cliCasesDir = join(REPO_ROOT, "ai_evals", "cases", "cli");
    return readdirSync(cliCasesDir)
      .filter((entry) => entry.endsWith(".json"))
      .sort((left, right) => left.localeCompare(right))
      .map((entry) => join(cliCasesDir, entry));
  }

  return [
    join(
      REPO_ROOT,
      "ai_evals",
      "cases",
      "frontend",
      `${surfaceToFrontendManifestName(surface)}.json`
    )
  ];
}

function surfaceToFrontendManifestName(
  surface: Exclude<EvalSurfaceName, "cli">
): "flow" | "app" | "script" {
  if (surface === "frontend-flow") {
    return "flow";
  }
  if (surface === "frontend-app") {
    return "app";
  }
  return "script";
}

function normalizeJudgeRubric(value: RawJudgeRubric | undefined): EvalJudgeRubric {
  return {
    minScore: value?.min_score
  };
}

function readRepoRelativeJson<T>(relativePath: string): T {
  return JSON.parse(readFileSync(resolveRepoRelativePath(relativePath), "utf8")) as T;
}

function resolveRepoRelativePath(relativePath: string): string {
  return join(REPO_ROOT, relativePath);
}

function assertNever(value: never): never {
  throw new Error(`Unexpected value: ${JSON.stringify(value)}`);
}
