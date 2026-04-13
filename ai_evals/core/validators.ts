import path from "node:path";
import ts from "typescript";
import type { BenchmarkCheck, FlowValidationSpec } from "./types";

export interface ScriptState {
  path: string;
  lang: string;
  args?: Record<string, unknown>;
  code: string;
}

export interface FlowState {
  summary?: string;
  value?: {
    preprocessor_module?: Record<string, unknown>;
    failure_module?: Record<string, unknown>;
    modules?: Array<Record<string, unknown>>;
    [key: string]: unknown;
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
const CONTROL_FLOW_MODULE_TYPES = new Set(["branchone", "branchall", "forloopflow", "whileloopflow"]);

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
  validate?: FlowValidationSpec;
}): BenchmarkCheck[] {
  const actualModules = getFlowModules(input.actual);
  const placeholderModuleIds = getInlineScriptPlaceholderModuleIds(input.actual);
  const checks: BenchmarkCheck[] = [
    check("flow has modules", actualModules.length > 0),
    check(
      "flow has no inline placeholder code",
      placeholderModuleIds.length === 0,
      placeholderModuleIds.length > 0
        ? `placeholder content in: ${placeholderModuleIds.join(", ")}`
        : undefined
    ),
  ];

  if (input.initial) {
    checks.push(
      check(
        "flow differs from initial",
        normalizeJson(input.actual) !== normalizeJson(input.initial)
      )
    );
  }

  if (input.expected) {
    checks.push(...validateFlowExpectedStructure(input.actual, input.expected));
  }

  if (input.validate) {
    checks.push(...validateFlowRequirements(input.actual, input.validate));
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
  const frontendSyntaxProblems = getAppFrontendSyntaxProblems(input.actual.frontend);
  const backendSyntaxProblems = getAppBackendSyntaxProblems(input.actual.backend);
  const unresolvedBackendRefs = getUnresolvedBackendReferences(
    input.actual.frontend,
    input.actual.backend
  );

  checks.push(check("app has frontend entrypoint", Boolean(input.actual.frontend["/index.tsx"])));
  checks.push(
    check(
      "app has non-empty frontend files",
      frontendEntries.some(([, content]) => content.trim().length > 0)
    )
  );
  checks.push(
    check(
      "frontend files have no syntax errors",
      frontendSyntaxProblems.length === 0,
      summarizeProblems(frontendSyntaxProblems)
    )
  );
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
  checks.push(
    check(
      "backend inline scripts have no syntax errors",
      backendSyntaxProblems.length === 0,
      summarizeProblems(backendSyntaxProblems)
    )
  );
  checks.push(
    check(
      "frontend backend references resolve",
      unresolvedBackendRefs.length === 0,
      summarizeProblems(unresolvedBackendRefs)
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
            cliFileContainsExpectedContent(actualContent, expectedContent)
          )
        );
      }
    }

    const expectedPaths = new Set(Object.keys(input.expectedFiles));
    const unexpectedPaths = Object.keys(input.actualFiles).filter((filePath) => !expectedPaths.has(filePath));
    checks.push(
      check(
        "workspace contains no unexpected files",
        unexpectedPaths.length === 0,
        summarizeProblems(unexpectedPaths)
      )
    );
  }

  if (input.initialFiles) {
    checks.push(check("workspace differs from initial", !fileMapsEqual(input.actualFiles, input.initialFiles)));
  }

  return checks;
}

function cliFileContainsExpectedContent(actualContent: string, expectedContent: string): boolean {
  const expectedSnippets = expectedContent
    .replace(/\r\n/g, "\n")
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.length > 0);

  if (expectedSnippets.length === 0) {
    return true;
  }

  const normalizedActual = actualContent.replace(/\r\n/g, "\n");

  return expectedSnippets.every((snippet) => normalizedActual.includes(snippet));
}

function check(name: string, passed: boolean, details?: string): BenchmarkCheck {
  return !passed && details ? { name, passed, details } : { name, passed };
}

function normalizeText(value: string): string {
  return value.replace(/\r\n/g, "\n").trim();
}

function normalizeJson(value: unknown): string {
  return JSON.stringify(value);
}

function summarizeProblems(problems: string[], limit = 5): string | undefined {
  if (problems.length === 0) {
    return undefined;
  }

  if (problems.length <= limit) {
    return problems.join("; ");
  }

  return `${problems.slice(0, limit).join("; ")}; ...and ${problems.length - limit} more`;
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

  return getTypeScriptSyntaxErrors(code, "eval.ts");
}

function getTypeScriptSyntaxErrors(code: string, fileName: string): string[] {
  const result = ts.transpileModule(code, {
    compilerOptions: {
      target: ts.ScriptTarget.ES2022,
      module: ts.ModuleKind.ESNext,
      jsx: ts.JsxEmit.ReactJSX,
    },
    reportDiagnostics: true,
    fileName,
  });

  return (result.diagnostics ?? []).map((diagnostic) =>
    ts.flattenDiagnosticMessageText(diagnostic.messageText, "\n")
  );
}

function getAppFrontendSyntaxProblems(frontend: Record<string, string>): string[] {
  const problems: string[] = [];

  for (const [filePath, content] of Object.entries(frontend)) {
    if (!isFrontendCodeFile(filePath)) {
      continue;
    }

    const errors = getTypeScriptSyntaxErrors(content, filePath);
    for (const error of errors) {
      problems.push(`${filePath}: ${error}`);
    }
  }

  return problems;
}

function getAppBackendSyntaxProblems(backend: Record<string, AppRunnableState>): string[] {
  const problems: string[] = [];

  for (const [key, runnable] of Object.entries(backend)) {
    if (runnable.type !== "inline") {
      continue;
    }

    const language = runnable.inlineScript?.language ?? "";
    const content = runnable.inlineScript?.content ?? "";
    for (const error of getScriptSyntaxErrors(content, language)) {
      problems.push(`${key}: ${error}`);
    }
  }

  return problems;
}

function isFrontendCodeFile(filePath: string): boolean {
  const extension = path.extname(filePath).toLowerCase();
  return extension === ".ts" || extension === ".tsx" || extension === ".js" || extension === ".jsx";
}

function getUnresolvedBackendReferences(
  frontend: Record<string, string>,
  backend: Record<string, AppRunnableState>
): string[] {
  const backendKeys = new Set(Object.keys(backend));
  const unresolved = new Set<string>();

  for (const [filePath, content] of Object.entries(frontend)) {
    for (const key of extractBackendCallKeys(content)) {
      if (!backendKeys.has(key)) {
        unresolved.add(`${filePath} references missing backend.${key}()`);
      }
    }
  }

  return [...unresolved];
}

function extractBackendCallKeys(content: string): string[] {
  const matches = content.matchAll(/\bbackend\.([A-Za-z_][A-Za-z0-9_]*)\s*\(/g);
  return [...new Set([...matches].map((match) => match[1]))];
}

function getFlowModules(flow: FlowState): Array<Record<string, unknown>> {
  return Array.isArray(flow.value?.modules) ? flow.value.modules : [];
}

function validateFlowExpectedStructure(
  actual: FlowState,
  expected: FlowState
): BenchmarkCheck[] {
  const checks: BenchmarkCheck[] = [];
  const expectedTopLevelModules = getFlowModules(expected);
  const actualTopLevelModules = getFlowModules(actual);

  const expectedSchemaFields = getTopLevelSchemaFields(expected.schema);
  if (expectedSchemaFields.length > 0) {
    checks.push(
      check(
        "flow schema includes expected top-level fields",
        expectedSchemaFields.every((field) => hasSchemaPath(actual.schema, field)),
        `missing one of: ${expectedSchemaFields.join(", ")}`
      )
    );
  }

  if (expectedTopLevelModules.length > 0) {
    const actualIds = actualTopLevelModules
      .map((module) => (typeof module.id === "string" ? module.id : null))
      .filter((id): id is string => Boolean(id));
    const expectedIds = expectedTopLevelModules
      .map((module) => (typeof module.id === "string" ? module.id : null))
      .filter((id): id is string => Boolean(id));

    checks.push(
      check(
        "flow includes expected top-level step ids",
        expectedIds.every((id) => actualIds.includes(id)),
        `expected ids: ${expectedIds.join(", ")}; actual ids: ${actualIds.join(", ")}`
      )
    );

    checks.push(
      check(
        "flow preserves expected top-level step order",
        preservesRelativeOrder(actualIds, expectedIds),
        `expected order: ${expectedIds.join(" -> ")}; actual ids: ${actualIds.join(" -> ")}`
      )
    );

    for (const expectedModule of expectedTopLevelModules) {
      const moduleId = typeof expectedModule.id === "string" ? expectedModule.id : null;
      if (!moduleId) {
        continue;
      }

      const actualModule = actualTopLevelModules.find((module) => module.id === moduleId);
      if (!actualModule) {
        continue;
      }

      const expectedType = getModuleType(expectedModule);
      if (expectedType && !(hasSuspendConfig(expectedModule) || hasSuspendConfig(actualModule))) {
        checks.push(
          check(
            `${moduleId} type matches expected`,
            getModuleType(actualModule) === expectedType,
            `expected ${expectedType}, got ${getModuleType(actualModule) ?? "(missing)"}`
          )
        );
      }

      const expectedPath = getModulePath(expectedModule);
      if (expectedPath) {
        checks.push(
          check(
            `${moduleId} path matches expected`,
            getModulePath(actualModule) === expectedPath,
            `expected ${expectedPath}, got ${getModulePath(actualModule) ?? "(missing)"}`
          )
        );
      }
    }
  }

  for (const specialModuleKey of ["preprocessor_module", "failure_module"] as const) {
    const expectedSpecialModule = getSpecialFlowModule(expected, specialModuleKey);
    if (!expectedSpecialModule) {
      continue;
    }

    const actualSpecialModule = getSpecialFlowModule(actual, specialModuleKey);
    checks.push(check(`${specialModuleKey} matches expected presence`, Boolean(actualSpecialModule)));

    if (!actualSpecialModule) {
      continue;
    }

    const expectedType = getModuleType(expectedSpecialModule);
    if (expectedType) {
      checks.push(
        check(
          `${specialModuleKey} type matches expected`,
          getModuleType(actualSpecialModule) === expectedType,
          `expected ${expectedType}, got ${getModuleType(actualSpecialModule) ?? "(missing)"}`
        )
      );
    }
  }

  return checks;
}

function validateFlowRequirements(
  flow: FlowState,
  validate: FlowValidationSpec
): BenchmarkCheck[] {
  const checks: BenchmarkCheck[] = [];

  for (const requiredPath of validate.schemaRequiredPaths ?? []) {
    checks.push(
      check(
        `schema includes ${requiredPath}`,
        hasSchemaPath(flow.schema, requiredPath),
        `missing schema path ${requiredPath}`
      )
    );
  }

  if (validate.schemaAnyOf && validate.schemaAnyOf.length > 0) {
    const matchingVariant = validate.schemaAnyOf.find((variant) =>
      variant.requiredPaths.every((requiredPath) => hasSchemaPath(flow.schema, requiredPath))
    );

    checks.push(
      check(
        "schema matches one accepted input shape",
        Boolean(matchingVariant),
        matchingVariant
          ? undefined
          : `expected one of: ${validate.schemaAnyOf
              .map((variant) => `[${variant.requiredPaths.join(", ")}]`)
              .join(" or ")}`
      )
    );
  }

  if (validate.resolveResultsRefs) {
    const unresolved = collectUnresolvedResultsRefs(flow);
    checks.push(
      check(
        "results references resolve",
        unresolved.length === 0,
        unresolved.length > 0 ? unresolved.join("; ") : undefined
      )
    );
  }

  for (const specialModule of validate.requireSpecialModules ?? []) {
    checks.push(
      check(
        `${specialModule} exists`,
        Boolean(getSpecialFlowModule(flow, specialModule))
      )
    );
  }

  for (const suspendStep of validate.requireSuspendSteps ?? []) {
    const module = findFlowModuleById(flow, suspendStep.id);
    checks.push(check(`${suspendStep.id} step exists`, Boolean(module)));
    if (!module) {
      continue;
    }

    checks.push(check(`${suspendStep.id} includes suspend config`, hasSuspendConfig(module)));
    if (!hasSuspendConfig(module)) {
      continue;
    }

    if (suspendStep.requiredEvents !== undefined) {
      checks.push(
        check(
          `${suspendStep.id} requires ${suspendStep.requiredEvents} approval event${suspendStep.requiredEvents === 1 ? "" : "s"}`,
          getSuspendRequiredEvents(module) === suspendStep.requiredEvents,
          `expected ${suspendStep.requiredEvents}, got ${getSuspendRequiredEvents(module) ?? "(missing)"}`
        )
      );
    }

    if (
      suspendStep.resumeRequiredStringFieldAnyOf &&
      suspendStep.resumeRequiredStringFieldAnyOf.length > 0
    ) {
      const stringFields = getSuspendResumeStringFields(module);
      checks.push(
        check(
          `${suspendStep.id} resume form includes one accepted comment field`,
          suspendStep.resumeRequiredStringFieldAnyOf.some((field) =>
            stringFields.includes(field)
          ),
          `expected one of [${suspendStep.resumeRequiredStringFieldAnyOf.join(", ")}], got [${stringFields.join(", ")}]`
        )
      );
    }
  }

  return checks;
}

function hasSchemaPath(schema: Record<string, unknown> | undefined, dottedPath: string): boolean {
  if (!schema || typeof schema !== "object") {
    return false;
  }

  const segments = dottedPath.split(".").filter(Boolean);
  if (segments.length === 0) {
    return false;
  }

  let current: Record<string, unknown> | undefined = schema;
  for (const segment of segments) {
    const properties = current?.properties;
    if (!properties || typeof properties !== "object") {
      return false;
    }

    const next = (properties as Record<string, unknown>)[segment];
    if (!next || typeof next !== "object") {
      return false;
    }
    current = next as Record<string, unknown>;
  }

  return true;
}

function getTopLevelSchemaFields(schema: Record<string, unknown> | undefined): string[] {
  if (!schema || typeof schema !== "object") {
    return [];
  }

  const properties = schema.properties;
  if (!properties || typeof properties !== "object") {
    return [];
  }

  return Object.keys(properties as Record<string, unknown>).filter((key) => key.length > 0);
}

function preservesRelativeOrder(actualIds: string[], expectedIds: string[]): boolean {
  if (expectedIds.length === 0) {
    return true;
  }

  let cursor = 0;
  for (const actualId of actualIds) {
    if (actualId === expectedIds[cursor]) {
      cursor += 1;
      if (cursor === expectedIds.length) {
        return true;
      }
    }
  }

  return false;
}

function collectUnresolvedResultsRefs(flow: FlowState): string[] {
  const unresolved = new Set<string>();
  validateModuleSequence(getFlowModules(flow), new Map<string, Record<string, unknown>>(), unresolved);
  return [...unresolved];
}

function validateModuleSequence(
  modules: Array<Record<string, unknown>>,
  parentVisibleModules: Map<string, Record<string, unknown>>,
  unresolved: Set<string>
): void {
  const visibleModules = new Map(parentVisibleModules);

  for (const module of modules) {
    validateResultsRefsInRecord(module, visibleModules, unresolved);
    validateNestedModuleResultsRefs(module, visibleModules, unresolved);

    if (typeof module.id === "string" && module.id.length > 0) {
      visibleModules.set(module.id, module);
    }
  }
}

function validateNestedModuleResultsRefs(
  module: Record<string, unknown>,
  visibleModules: Map<string, Record<string, unknown>>,
  unresolved: Set<string>
): void {
  const value = isObjectRecord(module.value) ? module.value : null;
  if (!value) {
    return;
  }

  const nestedSequences: Array<Array<Record<string, unknown>>> = [];

  if (Array.isArray(value.modules)) {
    nestedSequences.push(asModuleArray(value.modules));
  }

  if (Array.isArray(value.default)) {
    nestedSequences.push(asModuleArray(value.default));
  }

  if (Array.isArray(value.branches)) {
    for (const branch of value.branches) {
      if (!isObjectRecord(branch)) {
        continue;
      }
      if (typeof branch.expr === "string") {
        validateResultsRefsInExpression(
          branch.expr,
          `branch ${module.id ?? "(unnamed)"}`,
          visibleModules,
          unresolved
        );
      }
      if (Array.isArray(branch.modules)) {
        nestedSequences.push(asModuleArray(branch.modules));
      }
    }
  }

  for (const sequence of nestedSequences) {
    validateModuleSequence(sequence, visibleModules, unresolved);
  }
}

function validateResultsRefsInRecord(
  value: unknown,
  visibleModules: Map<string, Record<string, unknown>>,
  unresolved: Set<string>,
  context = "expression"
): void {
  if (typeof value === "string") {
    validateResultsRefsInExpression(value, context, visibleModules, unresolved);
    return;
  }

  if (Array.isArray(value)) {
    for (const entry of value) {
      validateResultsRefsInRecord(entry, visibleModules, unresolved, context);
    }
    return;
  }

  if (!isObjectRecord(value)) {
    return;
  }

  for (const [key, entry] of Object.entries(value)) {
    if (key === "content" || key === "modules" || key === "branches" || key === "default") {
      continue;
    }
    validateResultsRefsInRecord(entry, visibleModules, unresolved, key);
  }
}

function validateResultsRefsInExpression(
  expression: string,
  context: string,
  visibleModules: Map<string, Record<string, unknown>>,
  unresolved: Set<string>
): void {
  for (const ref of extractResultsRefs(expression)) {
    const module = visibleModules.get(ref.root);
    if (!module) {
      unresolved.add(`${context} references missing results.${ref.root}`);
      continue;
    }
    validateNestedResultsRefPath(ref.root, ref.path, module, context, unresolved);
  }
}

function extractResultsRefs(
  expression: string
): Array<{ root: string; path: string[] }> {
  const matches = expression.matchAll(/\bresults\.([A-Za-z0-9_-]+)((?:\.[A-Za-z0-9_-]+)*)/g);
  const refs = new Map<string, { root: string; path: string[] }>();

  for (const match of matches) {
    const root = match[1];
    const path = match[2]
      .split(".")
      .filter(Boolean);
    const key = `${root}:${path.join(".")}`;
    refs.set(key, { root, path });
  }

  return [...refs.values()];
}

function validateNestedResultsRefPath(
  rootId: string,
  path: string[],
  module: Record<string, unknown>,
  context: string,
  unresolved: Set<string>
): void {
  if (path.length === 0) {
    return;
  }

  const moduleType = getModuleType(module);
  if (!moduleType || !CONTROL_FLOW_MODULE_TYPES.has(moduleType)) {
    return;
  }

  const nestedIds = new Set(getImmediateNestedModuleIds(module));
  const [firstSegment] = path;
  if (nestedIds.has(firstSegment)) {
    unresolved.add(
      `${context} references nested results.${rootId}.${firstSegment} inside ${moduleType} ${rootId}`
    );
  }
}

function getAllFlowModules(flow: FlowState): Array<Record<string, unknown>> {
  const modules: Array<Record<string, unknown>> = [];
  const specialModules = ["preprocessor_module", "failure_module"] as const;

  for (const key of specialModules) {
    const specialModule = getSpecialFlowModule(flow, key);
    if (specialModule) {
      modules.push(specialModule);
      modules.push(...collectNestedModules(specialModule));
    }
  }

  for (const module of getFlowModules(flow)) {
    modules.push(module);
    modules.push(...collectNestedModules(module));
  }

  return modules;
}

function collectNestedModules(module: Record<string, unknown>): Array<Record<string, unknown>> {
  const nested: Array<Record<string, unknown>> = [];
  const value = isObjectRecord(module.value) ? module.value : null;
  if (!value) {
    return nested;
  }

  if (Array.isArray(value.modules)) {
    for (const child of asModuleArray(value.modules)) {
      nested.push(child, ...collectNestedModules(child));
    }
  }

  if (Array.isArray(value.default)) {
    for (const child of asModuleArray(value.default)) {
      nested.push(child, ...collectNestedModules(child));
    }
  }

  if (Array.isArray(value.branches)) {
    for (const branch of value.branches) {
      if (!isObjectRecord(branch) || !Array.isArray(branch.modules)) {
        continue;
      }
      for (const child of asModuleArray(branch.modules)) {
        nested.push(child, ...collectNestedModules(child));
      }
    }
  }

  return nested;
}

function findFlowModuleById(flow: FlowState, id: string): Record<string, unknown> | null {
  for (const module of getAllFlowModules(flow)) {
    if (module.id === id) {
      return module;
    }
  }
  return null;
}

function getInlineScriptPlaceholderModuleIds(flow: FlowState): string[] {
  return getAllFlowModules(flow).flatMap((module) => {
    const code = getModuleCode(module)?.trim();
    if (!code || !/^inline_script\.[A-Za-z0-9_-]+$/.test(code)) {
      return [];
    }

    if (typeof module.id === "string" && module.id.length > 0) {
      return [module.id];
    }

    return ["(unnamed)"];
  });
}

function getImmediateNestedModuleIds(module: Record<string, unknown>): string[] {
  const ids: string[] = [];
  const value = isObjectRecord(module.value) ? module.value : null;
  if (!value) {
    return ids;
  }

  if (Array.isArray(value.modules)) {
    ids.push(...asModuleArray(value.modules).flatMap((child) => (typeof child.id === "string" ? [child.id] : [])));
  }

  if (Array.isArray(value.default)) {
    ids.push(...asModuleArray(value.default).flatMap((child) => (typeof child.id === "string" ? [child.id] : [])));
  }

  if (Array.isArray(value.branches)) {
    for (const branch of value.branches) {
      if (!isObjectRecord(branch) || !Array.isArray(branch.modules)) {
        continue;
      }
      ids.push(
        ...asModuleArray(branch.modules).flatMap((child) => (typeof child.id === "string" ? [child.id] : []))
      );
    }
  }

  return ids;
}

function getModuleCode(module: Record<string, unknown>): string | null {
  const value = isObjectRecord(module.value) ? module.value : null;
  return typeof value?.content === "string" ? value.content : null;
}

function asModuleArray(value: unknown[]): Array<Record<string, unknown>> {
  return value.filter(isObjectRecord);
}

function isObjectRecord(value: unknown): value is Record<string, any> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
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

function getSuspendRequiredEvents(module: Record<string, unknown>): number | null {
  const suspend = isObjectRecord(module.suspend) ? module.suspend : null;
  return typeof suspend?.required_events === "number" ? suspend.required_events : null;
}

function getSuspendResumeStringFields(module: Record<string, unknown>): string[] {
  const suspend = isObjectRecord(module.suspend) ? module.suspend : null;
  const resumeForm = isObjectRecord(suspend?.resume_form) ? suspend.resume_form : null;
  const schema = isObjectRecord(resumeForm?.schema) ? resumeForm.schema : null;
  const properties = isObjectRecord(schema?.properties) ? schema.properties : null;
  if (!properties) {
    return [];
  }

  return Object.entries(properties).flatMap(([field, property]) => {
    if (!isObjectRecord(property) || property.type !== "string") {
      return [];
    }
    return [field];
  });
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
