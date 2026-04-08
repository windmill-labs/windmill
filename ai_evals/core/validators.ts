import ts from "typescript";
import type { BenchmarkCheck } from "./types";

export interface ScriptState {
  path: string;
  lang: string;
  args?: Record<string, unknown>;
  code: string;
}

export interface FlowState {
  value?: {
    modules?: Array<Record<string, unknown>>;
  };
  schema?: Record<string, unknown>;
}

export interface AppFilesState {
  frontend: Record<string, string>;
  backend: Record<string, AppRunnableState>;
}

export interface AppRunnableState {
  type?: string;
  name?: string;
  path?: string;
  inlineScript?: {
    language?: string;
    content?: string;
  };
}

const TS_LIKE_LANGUAGES = new Set(["bun", "deno", "nativets", "bunnative", "ts", "typescript"]);

export function validateScriptState(input: {
  actual: ScriptState;
  initial?: ScriptState;
  expected?: ScriptState;
}): BenchmarkCheck[] {
  const checks: BenchmarkCheck[] = [
    check("script exports entrypoint", hasSupportedEntrypoint(input.actual.code)),
    check("script has no syntax errors", getScriptSyntaxErrors(input.actual.code, input.actual.lang).length === 0),
  ];

  if (input.expected) {
    checks.push(
      check(
        "script path matches expected",
        input.actual.path === input.expected.path,
        `expected ${input.expected.path}, got ${input.actual.path}`
      )
    );
    checks.push(
      check(
        "script language matches expected",
        input.actual.lang === input.expected.lang,
        `expected ${input.expected.lang}, got ${input.actual.lang}`
      )
    );
    checks.push(
      check(
        "script code matches expected",
        normalizeText(input.actual.code) === normalizeText(input.expected.code)
      )
    );
  }

  if (input.initial) {
    checks.push(
      check(
        "script differs from initial",
        normalizeText(input.actual.code) !== normalizeText(input.initial.code)
      )
    );
  }

  return checks;
}

export function validateFlowState(input: {
  actual: FlowState;
  initial?: FlowState;
  expected?: FlowState;
}): BenchmarkCheck[] {
  const actualModules = getFlowModules(input.actual);
  const checks: BenchmarkCheck[] = [check("flow has modules", actualModules.length > 0)];

  if (input.initial) {
    checks.push(
      check(
        "flow differs from initial",
        normalizeJson(input.actual) !== normalizeJson(input.initial)
      )
    );
  }

  if (!input.expected) {
    return checks;
  }

  const expectedModules = getFlowModules(input.expected);
  const actualTypes = new Set(actualModules.map((module) => getModuleType(module) ?? ""));
  const expectedTypes = new Set(expectedModules.map((module) => getModuleType(module) ?? ""));
  const missingTypes = [...expectedTypes].filter((type) => type && !actualTypes.has(type));
  const actualTopLevelIds = getTopLevelFlowModuleIds(input.actual);
  const expectedTopLevelIds = getTopLevelFlowModuleIds(input.expected);
  const missingIds = expectedTopLevelIds.filter((id) => !actualTopLevelIds.includes(id));
  const expectedSchemaRootType = getSchemaRootType(input.expected.schema);
  const actualSchemaRootType = getSchemaRootType(input.actual.schema);
  const actualModuleById = new Map(
    actualModules
      .map((module) => [typeof module.id === "string" ? module.id : "", module] as const)
      .filter(([id]) => id.length > 0)
  );

  checks.push(
    check(
      "flow includes expected module types",
      missingTypes.length === 0,
      missingTypes.length > 0 ? `missing: ${missingTypes.join(", ")}` : undefined
    )
  );

  if (expectedSchemaRootType) {
    checks.push(
      check(
        "flow schema root type matches expected",
        actualSchemaRootType === expectedSchemaRootType,
        `expected ${expectedSchemaRootType}, got ${actualSchemaRootType ?? "(missing)"}`
      )
    );
  }

  if (expectedTopLevelIds.length > 0) {
    checks.push(
      check(
        "flow includes expected top-level step ids",
        missingIds.length === 0,
        missingIds.length > 0 ? `missing: ${missingIds.join(", ")}` : undefined
      )
    );
  }

  for (const expectedModule of expectedModules) {
    const expectedId = typeof expectedModule.id === "string" ? expectedModule.id : null;
    if (!expectedId) {
      continue;
    }

    const actualModule = actualModuleById.get(expectedId);
    if (!actualModule) {
      continue;
    }

    const expectedType = getModuleType(expectedModule);
    if (expectedType) {
      checks.push(
        check(
          `${expectedId} type matches expected`,
          getModuleType(actualModule) === expectedType,
          `expected ${expectedType}, got ${getModuleType(actualModule) ?? "(missing)"}`
        )
      );
    }

    const expectedPath = getModulePath(expectedModule);
    if (expectedPath) {
      checks.push(
        check(
          `${expectedId} path matches expected`,
          getModulePath(actualModule) === expectedPath,
          `expected ${expectedPath}, got ${getModulePath(actualModule) ?? "(missing)"}`
        )
      );
    }

    if (hasSuspendConfig(expectedModule)) {
      checks.push(check(`${expectedId} includes suspend config`, hasSuspendConfig(actualModule)));
    }

    const expectedContinueOnError = getContinueOnError(expectedModule);
    if (expectedContinueOnError !== undefined) {
      checks.push(
        check(
          `${expectedId} continue_on_error matches expected`,
          getContinueOnError(actualModule) === expectedContinueOnError
        )
      );
    }

    if (hasRetryConfig(expectedModule)) {
      checks.push(check(`${expectedId} includes retry config`, hasRetryConfig(actualModule)));
    }

    const expectedParallel = getModuleParallel(expectedModule);
    if (expectedParallel !== undefined) {
      checks.push(
        check(
          `${expectedId} parallel matches expected`,
          getModuleParallel(actualModule) === expectedParallel
        )
      );
    }
  }

  for (const specialKey of ["preprocessor_module", "failure_module"] as const) {
    const expectedSpecial = getSpecialFlowModule(input.expected, specialKey);
    if (!expectedSpecial) {
      continue;
    }

    const actualSpecial = getSpecialFlowModule(input.actual, specialKey);
    checks.push(check(`${specialKey} exists`, Boolean(actualSpecial)));
    if (!actualSpecial) {
      continue;
    }

    const expectedType = getModuleType(expectedSpecial);
    if (expectedType) {
      checks.push(
        check(
          `${specialKey} type matches expected`,
          getModuleType(actualSpecial) === expectedType,
          `expected ${expectedType}, got ${getModuleType(actualSpecial) ?? "(missing)"}`
        )
      );
    }

    if (hasRetryConfig(expectedSpecial)) {
      checks.push(check(`${specialKey} includes retry config`, hasRetryConfig(actualSpecial)));
    }
  }

  return checks;
}

export function validateAppState(input: {
  actual: AppFilesState;
  initial?: AppFilesState;
  expected?: AppFilesState;
}): BenchmarkCheck[] {
  const checks: BenchmarkCheck[] = [];
  const frontendEntries = Object.entries(input.actual.frontend ?? {});
  const backendEntries = Object.entries(input.actual.backend ?? {});

  checks.push(check("app has frontend entrypoint", Boolean(input.actual.frontend["/index.tsx"])));
  checks.push(check("app has non-empty frontend files", frontendEntries.some(([, content]) => content.trim().length > 0)));
  checks.push(
    check(
      "backend inline scripts have entrypoints",
      backendEntries.every(([, runnable]) => {
        if (runnable.type !== "inline") {
          return true;
        }
        return hasSupportedEntrypoint(runnable.inlineScript?.content ?? "");
      })
    )
  );

  if (input.initial) {
    checks.push(check("app differs from initial", !appStatesEqual(input.actual, input.initial)));
  }

  if (input.expected) {
    for (const [filePath, content] of Object.entries(input.expected.frontend)) {
      checks.push(
        check(
          `frontend includes ${filePath}`,
          normalizeText(input.actual.frontend[filePath] ?? "") === normalizeText(content)
        )
      );
    }
    for (const [runnableName, runnable] of Object.entries(input.expected.backend)) {
      const actualRunnable = input.actual.backend[runnableName];
      checks.push(check(`backend includes ${runnableName}`, Boolean(actualRunnable)));
      if (actualRunnable && runnable.inlineScript?.content) {
        checks.push(
          check(
            `${runnableName} code matches expected`,
            normalizeText(actualRunnable.inlineScript?.content ?? "") ===
              normalizeText(runnable.inlineScript.content)
          )
        );
      }
    }
  }

  return checks;
}

export function validateCliWorkspace(input: {
  actualFiles: Record<string, string>;
  expectedFiles?: Record<string, string>;
  initialFiles?: Record<string, string>;
}): BenchmarkCheck[] {
  const checks: BenchmarkCheck[] = [];

  if (input.expectedFiles) {
    for (const [filePath, expectedContent] of Object.entries(input.expectedFiles)) {
      const actualContent = input.actualFiles[filePath];
      checks.push(check(`creates ${filePath}`, actualContent !== undefined));
      if (actualContent !== undefined) {
        checks.push(
          check(
            `${filePath} contains expected content`,
            actualContent.includes(expectedContent.trim())
          )
        );
      }
    }
  }

  if (input.initialFiles) {
    checks.push(check("workspace differs from initial", !fileMapsEqual(input.actualFiles, input.initialFiles)));
  }

  return checks;
}

function check(name: string, passed: boolean, details?: string): BenchmarkCheck {
  return details ? { name, passed, details } : { name, passed };
}

function normalizeText(value: string): string {
  return value.replace(/\r\n/g, "\n").trim();
}

function normalizeJson(value: unknown): string {
  return JSON.stringify(value);
}

function hasSupportedEntrypoint(code: string): boolean {
  return (
    /export\s+(async\s+)?function\s+main\s*\(/.test(code) ||
    /export\s+default\s+(async\s+)?function\s*\(/.test(code)
  );
}

function getScriptSyntaxErrors(code: string, lang: string): string[] {
  if (!TS_LIKE_LANGUAGES.has(lang)) {
    return [];
  }

  const sourceFile = ts.createSourceFile("eval.ts", code, ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
  return sourceFile.parseDiagnostics.map((diagnostic) =>
    ts.flattenDiagnosticMessageText(diagnostic.messageText, "\n")
  );
}

function getFlowModules(flow: FlowState): Array<Record<string, unknown>> {
  return Array.isArray(flow.value?.modules) ? flow.value.modules : [];
}

function getTopLevelFlowModuleIds(flow: FlowState): string[] {
  return getFlowModules(flow)
    .map((module) => module.id)
    .filter((value): value is string => typeof value === "string");
}

function getSpecialFlowModule(
  flow: FlowState,
  key: "preprocessor_module" | "failure_module"
): Record<string, unknown> | null {
  if (!flow.value || typeof flow.value !== "object") {
    return null;
  }
  const module = (flow.value as Record<string, unknown>)[key];
  return module && typeof module === "object" ? (module as Record<string, unknown>) : null;
}

function getModuleType(module: Record<string, unknown>): string | null {
  const value = module.value;
  if (!value || typeof value !== "object") {
    return null;
  }
  return typeof (value as Record<string, unknown>).type === "string"
    ? ((value as Record<string, string>).type)
    : null;
}

function getModulePath(module: Record<string, unknown>): string | null {
  const value = module.value;
  if (!value || typeof value !== "object") {
    return null;
  }
  return typeof (value as Record<string, unknown>).path === "string"
    ? ((value as Record<string, string>).path)
    : null;
}

function hasSuspendConfig(module: Record<string, unknown>): boolean {
  return typeof module.suspend === "object" && module.suspend !== null;
}

function getContinueOnError(module: Record<string, unknown>): boolean | undefined {
  return typeof module.continue_on_error === "boolean"
    ? (module.continue_on_error as boolean)
    : undefined;
}

function hasRetryConfig(module: Record<string, unknown>): boolean {
  return typeof module.retry === "object" && module.retry !== null;
}

function getModuleParallel(module: Record<string, unknown>): boolean | undefined {
  const value = module.value;
  if (!value || typeof value !== "object") {
    return undefined;
  }
  return typeof (value as Record<string, unknown>).parallel === "boolean"
    ? ((value as Record<string, boolean>).parallel)
    : undefined;
}

function getSchemaRootType(schema: Record<string, unknown> | undefined): string | null {
  if (!schema || typeof schema !== "object") {
    return null;
  }
  const properties = (schema.properties ?? {}) as Record<string, unknown>;
  const root = properties["root"];
  if (!root || typeof root !== "object") {
    return null;
  }
  return typeof (root as { type?: unknown }).type === "string"
    ? ((root as { type: string }).type)
    : null;
}

function appStatesEqual(left: AppFilesState, right: AppFilesState): boolean {
  return fileMapsEqual(left.frontend, right.frontend) && fileMapsEqual(stringifyBackend(left.backend), stringifyBackend(right.backend));
}

function stringifyBackend(backend: Record<string, AppRunnableState>): Record<string, string> {
  const result: Record<string, string> = {};
  for (const [key, value] of Object.entries(backend)) {
    result[key] = JSON.stringify(value);
  }
  return result;
}

function fileMapsEqual(left: Record<string, string>, right: Record<string, string>): boolean {
  const leftEntries = Object.entries(left).sort(([a], [b]) => a.localeCompare(b));
  const rightEntries = Object.entries(right).sort(([a], [b]) => a.localeCompare(b));
  if (leftEntries.length !== rightEntries.length) {
    return false;
  }
  return leftEntries.every(([key, value], index) => {
    const [otherKey, otherValue] = rightEntries[index];
    return key === otherKey && normalizeText(value) === normalizeText(otherValue);
  });
}
