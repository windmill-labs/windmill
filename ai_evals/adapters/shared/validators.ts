import ts from "typescript";

export interface BenchmarkCheck {
  name: string;
  passed: boolean;
  required?: boolean;
  details?: string;
}

interface ScriptLikeArtifact {
  code: string;
  lang: string;
  path: string;
}

interface FlowLikeArtifact {
  value?: {
    modules?: Array<Record<string, unknown>>;
  };
  schema?: Record<string, unknown>;
}

interface AppLikeFiles {
  frontend: Record<string, string>;
  backend: Record<string, AppLikeBackendRunnable>;
}

interface AppLikeBackendRunnable {
  type?: string;
  name?: string;
  path?: string;
  inlineScript?: {
    language?: string;
    content?: string;
  };
}

interface CliExpectedFileCheck {
  path: string;
  mustContain?: string[];
  mustNotContain?: string[];
}

interface CliFileArtifactResult {
  path: string;
  exists: boolean;
  content?: string;
}

const TS_LIKE_LANGUAGES = new Set(["bun", "deno", "nativets", "bunnative", "ts", "typescript"]);

export function requiredCheck(
  name: string,
  passed: boolean,
  details?: string
): BenchmarkCheck {
  return {
    name,
    passed,
    required: true,
    ...(details ? { details } : {})
  };
}

export function optionalCheck(
  name: string,
  passed: boolean,
  details?: string
): BenchmarkCheck {
  return {
    name,
    passed,
    required: false,
    ...(details ? { details } : {})
  };
}

export function allRequiredChecksPassed(checks: BenchmarkCheck[]): boolean {
  return checks.every((check) => check.required === false || check.passed);
}

export function getRequiredFailedChecks(checks: BenchmarkCheck[]): string[] {
  return checks
    .filter((check) => check.required !== false && !check.passed)
    .map((check) => check.name);
}

export function buildJudgeChecks(input: {
  evaluationResult:
    | {
        success: boolean;
        resemblanceScore: number;
        error?: string;
      }
    | undefined;
  minJudgeScore: number;
}): BenchmarkCheck[] {
  return [
    requiredCheck(
      "judge evaluation succeeded",
      Boolean(input.evaluationResult?.success),
      input.evaluationResult?.error
    ),
    requiredCheck(
      `judge score >= ${input.minJudgeScore}`,
      (input.evaluationResult?.resemblanceScore ?? 0) >= input.minJudgeScore,
      `score=${input.evaluationResult?.resemblanceScore ?? 0}`
    )
  ];
}

export function validateCliArtifact(input: {
  assistantOutput: string;
  skillsInvoked: string[];
  expectedSkill?: string;
  expectedOutputSubstrings?: string[];
  expectedFiles: CliExpectedFileCheck[];
  fileResults: CliFileArtifactResult[];
}): BenchmarkCheck[] {
  const checks: BenchmarkCheck[] = [];

  if (input.expectedSkill) {
    checks.push(
      requiredCheck(
        `invokes ${input.expectedSkill}`,
        input.skillsInvoked.includes(input.expectedSkill),
        `skills invoked: ${input.skillsInvoked.join(", ")}`
      )
    );
  }

  for (const expectedOutput of input.expectedOutputSubstrings ?? []) {
    checks.push(
      requiredCheck(
        `mentions '${expectedOutput}' in assistant output`,
        input.assistantOutput.includes(expectedOutput)
      )
    );
  }

  for (const expectedFile of input.expectedFiles) {
    const fileResult = input.fileResults.find((entry) => entry.path === expectedFile.path);
    const content = fileResult?.content ?? "";

    checks.push(
      requiredCheck(`creates ${expectedFile.path}`, Boolean(fileResult?.exists))
    );

    for (const requiredSnippet of expectedFile.mustContain ?? []) {
      checks.push(
        requiredCheck(
          `${expectedFile.path} contains '${requiredSnippet}'`,
          content.includes(requiredSnippet)
        )
      );
    }

    for (const forbiddenSnippet of expectedFile.mustNotContain ?? []) {
      checks.push(
        requiredCheck(
          `${expectedFile.path} avoids '${forbiddenSnippet}'`,
          !content.includes(forbiddenSnippet)
        )
      );
    }
  }

  return checks;
}

export function validateScriptArtifact(input: {
  generatedScript: ScriptLikeArtifact;
  expectedScript: ScriptLikeArtifact;
  initialScript?: ScriptLikeArtifact;
}): BenchmarkCheck[] {
  const lintErrors = getScriptLintErrors(input.generatedScript.code, input.generatedScript.lang);
  const normalizedGenerated = normalizeText(input.generatedScript.code);
  const normalizedInitial = input.initialScript
    ? normalizeText(input.initialScript.code)
    : null;

  return [
    requiredCheck(
      "script path matches expected",
      input.generatedScript.path === input.expectedScript.path,
      `expected ${input.expectedScript.path}, got ${input.generatedScript.path}`
    ),
    requiredCheck(
      "script language matches expected",
      input.generatedScript.lang === input.expectedScript.lang,
      `expected ${input.expectedScript.lang}, got ${input.generatedScript.lang}`
    ),
    requiredCheck(
      "script exports entrypoint",
      hasSupportedEntrypoint(input.generatedScript.code)
    ),
    requiredCheck(
      "script has no syntax errors",
      lintErrors.length === 0,
      lintErrors.join(" | ")
    ),
    ...(normalizedInitial === null
      ? []
      : [
          requiredCheck(
            "script differs from initial input",
            normalizedGenerated !== normalizedInitial
          )
        ])
  ];
}

export function validateFlowArtifact(input: {
  generatedFlow: FlowLikeArtifact;
  expectedFlow: FlowLikeArtifact;
}): BenchmarkCheck[] {
  const generatedModules = getFlowModules(input.generatedFlow);
  const expectedModules = getFlowModules(input.expectedFlow);
  const generatedTypes = collectFlowModuleTypes(generatedModules);
  const expectedTypes = collectFlowModuleTypes(expectedModules);
  const missingTypes = [...expectedTypes].filter((type) => !generatedTypes.has(type));
  const generatedTopLevelIds = getTopLevelFlowModuleIds(input.generatedFlow);
  const expectedTopLevelIds = getTopLevelFlowModuleIds(input.expectedFlow);
  const missingTopLevelIds = expectedTopLevelIds.filter(
    (id) => !generatedTopLevelIds.includes(id)
  );
  const expectedSchemaType = getSchemaRootType(input.expectedFlow.schema);
  const generatedSchemaType = getSchemaRootType(input.generatedFlow.schema);

  return [
    requiredCheck("flow has modules", generatedModules.length > 0),
    requiredCheck(
      "flow includes expected module types",
      missingTypes.length === 0,
      missingTypes.length > 0 ? `missing types: ${missingTypes.join(", ")}` : undefined
    ),
    ...(expectedSchemaType
      ? [
          requiredCheck(
            "flow schema root type matches expected",
            generatedSchemaType === expectedSchemaType,
            `expected ${expectedSchemaType}, got ${generatedSchemaType ?? "(missing)"}`
          )
        ]
      : []),
    optionalCheck(
      "flow includes expected top-level step ids",
      missingTopLevelIds.length === 0,
      missingTopLevelIds.length > 0
        ? `missing ids: ${missingTopLevelIds.join(", ")}`
        : undefined
    )
  ];
}

export function validateAppArtifact(input: {
  generatedApp: AppLikeFiles;
  initialApp?: AppLikeFiles;
}): BenchmarkCheck[] {
  const frontendEntries = Object.entries(input.generatedApp.frontend ?? {});
  const emptyFrontendFiles = frontendEntries
    .filter(([, content]) => normalizeText(content).length === 0)
    .map(([path]) => path);
  const backendReferenceKeys = collectBackendReferences(
    frontendEntries.map(([, content]) => content)
  );
  const missingBackendReferences = backendReferenceKeys.filter(
    (key) => input.generatedApp.backend[key] === undefined
  );
  const invalidInlineRunnables = Object.entries(input.generatedApp.backend ?? {})
    .filter(([, runnable]) => runnable.type === "inline")
    .filter(([, runnable]) => !hasSupportedEntrypoint(runnable.inlineScript?.content ?? ""))
    .map(([key]) => key);
  const hasChangedFromInitial = input.initialApp
    ? !appFilesEqual(input.generatedApp, input.initialApp)
    : true;

  return [
    requiredCheck("app has frontend files", frontendEntries.length > 0),
    requiredCheck(
      "app has frontend entrypoint",
      Object.keys(input.generatedApp.frontend ?? {}).some(
        (filePath) => filePath === "/index.tsx" || filePath === "/index.jsx"
      )
    ),
    requiredCheck(
      "frontend files are non-empty",
      emptyFrontendFiles.length === 0,
      emptyFrontendFiles.length > 0
        ? `empty files: ${emptyFrontendFiles.join(", ")}`
        : undefined
    ),
    requiredCheck(
      "frontend backend references resolve",
      missingBackendReferences.length === 0,
      missingBackendReferences.length > 0
        ? `missing runnables: ${missingBackendReferences.join(", ")}`
        : undefined
    ),
    requiredCheck(
      "inline backend runnables export entrypoint",
      invalidInlineRunnables.length === 0,
      invalidInlineRunnables.length > 0
        ? `invalid inline runnables: ${invalidInlineRunnables.join(", ")}`
        : undefined
    ),
    ...(input.initialApp
      ? [requiredCheck("app differs from initial input", hasChangedFromInitial)]
      : [])
  ];
}

function hasSupportedEntrypoint(code: string): boolean {
  return (
    /export\s+(async\s+)?function\s+main\s*\(/.test(code) ||
    /export\s+(async\s+)?function\s+preprocessor\s*\(/.test(code)
  );
}

function getScriptLintErrors(code: string, lang: string): string[] {
  if (!TS_LIKE_LANGUAGES.has(lang)) {
    return hasSupportedEntrypoint(code)
      ? []
      : ["Script must export a main or preprocessor function."];
  }

  const output = ts.transpileModule(code, {
    compilerOptions: {
      target: ts.ScriptTarget.ES2022,
      module: ts.ModuleKind.ESNext,
      moduleResolution: ts.ModuleResolutionKind.Bundler,
      noEmit: true,
      allowJs: true,
      checkJs: false,
      strict: false,
      skipLibCheck: true
    },
    fileName: "script.ts",
    reportDiagnostics: true
  });

  const diagnostics = (output.diagnostics ?? []).map((diagnostic) =>
    ts.flattenDiagnosticMessageText(diagnostic.messageText, "\n")
  );

  if (!hasSupportedEntrypoint(code)) {
    diagnostics.push("Script must export a main or preprocessor function.");
  }

  return diagnostics;
}

function getFlowModules(flow: FlowLikeArtifact): Array<Record<string, unknown>> {
  const rootModules = Array.isArray(flow.value?.modules) ? flow.value.modules : [];
  const collected: Array<Record<string, unknown>> = [];

  for (const module of rootModules) {
    visitFlowModule(module, collected);
  }

  return collected;
}

function visitFlowModule(
  module: Record<string, unknown>,
  collected: Array<Record<string, unknown>>
): void {
  collected.push(module);

  const value = asRecord(module.value);
  const nestedModules = Array.isArray(value?.modules) ? value.modules : [];
  for (const nested of nestedModules) {
    if (isRecord(nested)) {
      visitFlowModule(nested, collected);
    }
  }

  const branches = Array.isArray(value?.branches) ? value.branches : [];
  for (const branch of branches) {
    const branchRecord = asRecord(branch);
    const branchModules = Array.isArray(branchRecord?.modules)
      ? branchRecord.modules
      : [];
    for (const nested of branchModules) {
      if (isRecord(nested)) {
        visitFlowModule(nested, collected);
      }
    }
  }

  const defaultModules = Array.isArray(value?.default) ? value.default : [];
  for (const nested of defaultModules) {
    if (isRecord(nested)) {
      visitFlowModule(nested, collected);
    }
  }
}

function collectFlowModuleTypes(
  modules: Array<Record<string, unknown>>
): Set<string> {
  const types = new Set<string>();
  for (const module of modules) {
    const value = asRecord(module.value);
    if (typeof value?.type === "string") {
      types.add(value.type);
    }
  }
  return types;
}

function getTopLevelFlowModuleIds(flow: FlowLikeArtifact): string[] {
  const rootModules = Array.isArray(flow.value?.modules) ? flow.value.modules : [];
  return rootModules
    .map((module) => (isRecord(module) && typeof module.id === "string" ? module.id : null))
    .filter((id): id is string => id !== null);
}

function getSchemaRootType(schema: Record<string, unknown> | undefined): string | null {
  return typeof schema?.type === "string" ? schema.type : null;
}

function collectBackendReferences(frontendContents: string[]): string[] {
  const references = new Set<string>();
  const backendCallPattern = /backend\.([A-Za-z0-9_]+)\s*\(/g;

  for (const content of frontendContents) {
    for (const match of content.matchAll(backendCallPattern)) {
      const key = match[1];
      if (key) {
        references.add(key);
      }
    }
  }

  return [...references].sort((left, right) => left.localeCompare(right));
}

function appFilesEqual(left: AppLikeFiles, right: AppLikeFiles): boolean {
  return stableStringify(left) === stableStringify(right);
}

function stableStringify(value: unknown): string {
  return JSON.stringify(sortJsonValue(value));
}

function sortJsonValue(value: unknown): unknown {
  if (Array.isArray(value)) {
    return value.map(sortJsonValue);
  }

  if (!isRecord(value)) {
    return value;
  }

  return Object.fromEntries(
    Object.entries(value)
      .sort((left, right) => left[0].localeCompare(right[0]))
      .map(([key, nestedValue]) => [key, sortJsonValue(nestedValue)])
  );
}

function normalizeText(value: string): string {
  return value.replace(/\r\n/g, "\n").trim();
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function asRecord(value: unknown): Record<string, unknown> | null {
  return isRecord(value) ? value : null;
}
