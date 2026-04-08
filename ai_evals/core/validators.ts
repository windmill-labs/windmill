import ts from "typescript";
import type { BenchmarkCheck, FlowModuleValidation, FlowValidationSpec } from "./types";

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
  validate?: FlowValidationSpec;
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
    if (input.validate) {
      checks.push(...validateFlowRequirements(input.actual, input.validate));
    }
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
  return !passed && details ? { name, passed, details } : { name, passed };
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

  const result = ts.transpileModule(code, {
    compilerOptions: {
      target: ts.ScriptTarget.ES2022,
      module: ts.ModuleKind.ESNext,
    },
    reportDiagnostics: true,
    fileName: "eval.ts",
  });

  return (result.diagnostics ?? []).map((diagnostic) =>
    ts.flattenDiagnosticMessageText(diagnostic.messageText, "\n")
  );
}

function getFlowModules(flow: FlowState): Array<Record<string, unknown>> {
  return Array.isArray(flow.value?.modules) ? flow.value.modules : [];
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

  for (const requirement of validate.modules ?? []) {
    const matched = findMatchingFlowModule(flow, requirement);
    checks.push(
      check(
        requirement.name,
        Boolean(matched),
        matched ? undefined : describeMissingModuleRequirement(requirement)
      )
    );
  }

  return checks;
}

function getTopLevelFlowModuleIds(flow: FlowState): string[] {
  return getFlowModules(flow)
    .map((module) => module.id)
    .filter((value): value is string => typeof value === "string");
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

function findMatchingFlowModule(
  flow: FlowState,
  requirement: FlowModuleValidation
): Record<string, unknown> | null {
  for (const module of getAllFlowModules(flow)) {
    if (matchesFlowModuleRequirement(module, requirement)) {
      return module;
    }
  }
  return null;
}

function matchesFlowModuleRequirement(
  module: Record<string, unknown>,
  requirement: FlowModuleValidation
): boolean {
  if (requirement.typeAnyOf && requirement.typeAnyOf.length > 0) {
    const actualType = getModuleType(module);
    if (!actualType || !requirement.typeAnyOf.includes(actualType)) {
      return false;
    }
  }

  const inputRefs = getModuleInputRefs(module);
  if (requirement.inputRefsAny && requirement.inputRefsAny.length > 0) {
    if (!requirement.inputRefsAny.some((ref) => inputRefs.includes(ref))) {
      return false;
    }
  }

  if (requirement.inputRefsAll && requirement.inputRefsAll.length > 0) {
    if (!requirement.inputRefsAll.every((ref) => inputRefs.includes(ref))) {
      return false;
    }
  }

  if (requirement.inputRefsContainAny && requirement.inputRefsContainAny.length > 0) {
    if (!requirement.inputRefsContainAny.some((snippet) => inputRefs.some((ref) => ref.includes(snippet)))) {
      return false;
    }
  }

  if (requirement.inputRefsContainAll && requirement.inputRefsContainAll.length > 0) {
    if (!requirement.inputRefsContainAll.every((snippet) => inputRefs.some((ref) => ref.includes(snippet)))) {
      return false;
    }
  }

  const code = getModuleCode(module);
  if (requirement.codeContainsAny && requirement.codeContainsAny.length > 0) {
    if (!code || !requirement.codeContainsAny.some((snippet) => code.includes(snippet))) {
      return false;
    }
  }

  if (requirement.codeContainsAll && requirement.codeContainsAll.length > 0) {
    if (!code || !requirement.codeContainsAll.every((snippet) => code.includes(snippet))) {
      return false;
    }
  }

  if (requirement.codeRegexAll && requirement.codeRegexAll.length > 0) {
    if (!code) {
      return false;
    }
    for (const pattern of requirement.codeRegexAll) {
      const regex = new RegExp(pattern, "m");
      if (!regex.test(code)) {
        return false;
      }
    }
  }

  return true;
}

function describeMissingModuleRequirement(requirement: FlowModuleValidation): string {
  const parts: string[] = [];
  if (requirement.typeAnyOf?.length) {
    parts.push(`type in [${requirement.typeAnyOf.join(", ")}]`);
  }
  if (requirement.inputRefsAny?.length) {
    parts.push(`any input refs [${requirement.inputRefsAny.join(", ")}]`);
  }
  if (requirement.inputRefsAll?.length) {
    parts.push(`all input refs [${requirement.inputRefsAll.join(", ")}]`);
  }
  if (requirement.inputRefsContainAny?.length) {
    parts.push(`any input refs containing [${requirement.inputRefsContainAny.join(", ")}]`);
  }
  if (requirement.inputRefsContainAll?.length) {
    parts.push(`all input refs containing [${requirement.inputRefsContainAll.join(", ")}]`);
  }
  if (requirement.codeContainsAny?.length) {
    parts.push(`code contains any [${requirement.codeContainsAny.join(", ")}]`);
  }
  if (requirement.codeContainsAll?.length) {
    parts.push(`code contains all [${requirement.codeContainsAll.join(", ")}]`);
  }
  if (requirement.codeRegexAll?.length) {
    parts.push(`code matches [${requirement.codeRegexAll.join(", ")}]`);
  }
  return parts.join("; ");
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

function getModuleInputRefs(module: Record<string, unknown>): string[] {
  const value = isObjectRecord(module.value) ? module.value : null;
  const inputTransforms = isObjectRecord(value?.input_transforms) ? value?.input_transforms : null;
  if (!inputTransforms) {
    return [];
  }

  return Object.values(inputTransforms)
    .flatMap((entry) => {
      if (!isObjectRecord(entry) || typeof entry.expr !== "string") {
        return [];
      }
      return [entry.expr.trim()];
    });
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
