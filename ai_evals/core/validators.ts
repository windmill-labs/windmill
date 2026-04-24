import path from "node:path";
import ts from "typescript";
import type {
  AppValidationSpec,
  BenchmarkCheck,
  CliTrace,
  CliValidationSpec,
  FlowValidationSpec,
} from "./types";

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
  datatables: AppDatatableState[];
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

export interface AppDatatableState {
  datatable_name: string;
  schemas: Record<string, Record<string, Record<string, string>>>;
  error?: string;
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
  validate?: AppValidationSpec;
  toolsUsed?: string[];
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

    for (const expectedDatatable of input.expected.datatables ?? []) {
      const expectedSchemas = Object.entries(expectedDatatable.schemas ?? {});
      if (expectedSchemas.length === 0) {
        checks.push(
          check(
            `datatable ${expectedDatatable.datatable_name} exists`,
            (input.actual.datatables ?? []).some(
              (datatable) => datatable.datatable_name === expectedDatatable.datatable_name
            )
          )
        );
        continue;
      }

      for (const [schemaName, tables] of expectedSchemas) {
        const tableNames = Object.keys(tables ?? {});
        for (const tableName of tableNames) {
          checks.push(
            check(
              `datatable table ${expectedDatatable.datatable_name}/${schemaName}.${tableName} exists`,
              hasDatatableTable(input.actual.datatables ?? [], {
                datatableName: expectedDatatable.datatable_name,
                schema: schemaName,
                table: tableName,
              })
            )
          );
        }
      }
    }
  }

  if (input.validate) {
    checks.push(...validateAppRequirements(input.actual, input.validate, input.toolsUsed ?? []));
  }

  return checks;
}

export function validateCliWorkspace(input: {
  actualFiles: Record<string, string>;
  expectedFiles?: Record<string, string>;
  initialFiles?: Record<string, string>;
  assistantOutput?: string;
  trace?: CliTrace;
  cliExpect?: CliValidationSpec;
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

  if (input.cliExpect?.workspaceUnchanged) {
    const baselineFiles = input.initialFiles ?? {};
    checks.push(
      check(
        "workspace remains unchanged",
        fileMapsEqual(input.actualFiles, baselineFiles),
        summarizeWorkspaceDiff(input.actualFiles, baselineFiles)
      )
    );
  } else if (input.initialFiles) {
    checks.push(check("workspace differs from initial", !fileMapsEqual(input.actualFiles, input.initialFiles)));
  }

  if (input.cliExpect) {
    checks.push(
      ...validateCliExpectations(input.assistantOutput ?? "", input.trace, input.cliExpect)
    );
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

function validateCliExpectations(
  assistantOutput: string,
  trace: CliTrace | undefined,
  cliExpect: CliValidationSpec
): BenchmarkCheck[] {
  const checks: BenchmarkCheck[] = [];

  if (!trace) {
    checks.push(check("cli trace is available", false));
    return checks;
  }

  for (const skillName of cliExpect.requiredSkills ?? []) {
    checks.push(
      check(
        `invokes skill ${skillName}`,
        cliSkillWasInvoked(trace, skillName),
        `skills=${trace.skillsInvoked.join(", ") || "(none)"}`
      )
    );
  }

  for (const skillName of cliExpect.forbiddenSkills ?? []) {
    checks.push(
      check(
        `does not invoke skill ${skillName}`,
        !cliSkillWasInvoked(trace, skillName),
        `skills=${trace.skillsInvoked.join(", ") || "(none)"}`
      )
    );
  }

  for (const skillName of cliExpect.requiredSkillsBeforeFirstMutation ?? []) {
    const firstSkillIndex = getFirstSkillToolIndex(trace, skillName);
    const firstMutationIndex = trace.firstMutationToolIndex;
    checks.push(
      check(
        `invokes skill ${skillName} before first mutation`,
        firstSkillIndex !== null &&
          firstMutationIndex !== null &&
          firstSkillIndex < firstMutationIndex,
        `firstSkillIndex=${firstSkillIndex ?? "none"}; firstMutationIndex=${firstMutationIndex ?? "none"}`
      )
    );
  }

  for (const phrase of cliExpect.requiredAssistantMentions ?? []) {
    checks.push(
      check(
        `assistant mentions ${phrase}`,
        assistantMentions(assistantOutput, phrase)
      )
    );
  }

  for (const phrase of cliExpect.forbiddenAssistantMentions ?? []) {
    checks.push(
      check(
        `assistant does not mention ${phrase}`,
        !assistantMentions(assistantOutput, phrase)
      )
    );
  }

  if ((cliExpect.orderedAssistantMentions?.length ?? 0) > 0) {
    checks.push(
      check(
        "assistant mentions expected items in order",
        stringsAppearInOrder(assistantOutput, cliExpect.orderedAssistantMentions!),
        `ordered=${cliExpect.orderedAssistantMentions!.join(" -> ")}`
      )
    );
  }

  for (const command of cliExpect.requiredProposedCommands ?? []) {
    checks.push(
      check(
        `assistant proposes ${command}`,
        commandListContains(trace.proposedCommands, command),
        `proposed=${trace.proposedCommands.join("; ") || "(none)"}`
      )
    );
  }

  for (const command of cliExpect.forbiddenProposedCommands ?? []) {
    checks.push(
      check(
        `assistant does not propose ${command}`,
        !commandListContains(trace.proposedCommands, command),
        `proposed=${trace.proposedCommands.join("; ") || "(none)"}`
      )
    );
  }

  if ((cliExpect.orderedProposedCommands?.length ?? 0) > 0) {
    checks.push(
      check(
        "assistant proposes expected commands in order",
        stringsAppearInOrder(trace.proposedCommands.join("\n"), cliExpect.orderedProposedCommands!),
        `ordered=${cliExpect.orderedProposedCommands!.join(" -> ")}; proposed=${trace.proposedCommands.join("; ") || "(none)"}`
      )
    );
  }

  for (const pattern of cliExpect.forbiddenExecutedCommands ?? []) {
    checks.push(
      check(
        `does not execute ${pattern}`,
        !trace.executedWmillCommands.some((command) => matchesCommandPattern(command, pattern)),
        `executed=${trace.executedWmillCommands.join("; ") || "(none)"}`
      )
    );
  }

  return checks;
}

function cliSkillWasInvoked(trace: CliTrace, skillName: string): boolean {
  return trace.skillsInvoked.some((skill) => skill === skillName);
}

function getFirstSkillToolIndex(trace: CliTrace, skillName: string): number | null {
  for (const [index, tool] of trace.toolsUsed.entries()) {
    if (tool.tool !== "Skill") {
      continue;
    }

    const inputSkill = typeof tool.input.skill === "string" ? tool.input.skill : null;
    if (inputSkill === skillName) {
      return index;
    }
  }

  return null;
}

function assistantMentions(output: string, phrase: string): boolean {
  return output.toLowerCase().includes(phrase.toLowerCase());
}

function stringsAppearInOrder(output: string, phrases: string[]): boolean {
  const normalizedOutput = output.toLowerCase();
  let startIndex = 0;

  for (const phrase of phrases) {
    const index = normalizedOutput.indexOf(phrase.toLowerCase(), startIndex);
    if (index === -1) {
      return false;
    }
    startIndex = index + phrase.length;
  }

  return true;
}

function commandListContains(commands: string[], command: string): boolean {
  const normalizedNeedle = command.toLowerCase();
  return commands.some((entry) => entry.toLowerCase().includes(normalizedNeedle));
}

function matchesCommandPattern(command: string, pattern: string): boolean {
  try {
    return new RegExp(pattern, "i").test(command);
  } catch {
    return command.toLowerCase().includes(pattern.toLowerCase());
  }
}

function summarizeWorkspaceDiff(
  actualFiles: Record<string, string>,
  baselineFiles: Record<string, string>
): string | undefined {
  const changes: string[] = [];

  for (const filePath of Object.keys(actualFiles)) {
    if (!(filePath in baselineFiles)) {
      changes.push(`added ${filePath}`);
    } else if (actualFiles[filePath] !== baselineFiles[filePath]) {
      changes.push(`changed ${filePath}`);
    }
  }

  for (const filePath of Object.keys(baselineFiles)) {
    if (!(filePath in actualFiles)) {
      changes.push(`removed ${filePath}`);
    }
  }

  return summarizeProblems(changes);
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
  const actualTopLevelModules = getFlowModules(flow);
  const actualIds = actualTopLevelModules
    .map((module) => (typeof module.id === "string" ? module.id : null))
    .filter((id): id is string => Boolean(id));

  if (validate.exactTopLevelStepIds && validate.exactTopLevelStepIds.length > 0) {
    checks.push(
      check(
        "flow top-level step ids match exactly",
        stringArraysEqual(actualIds, validate.exactTopLevelStepIds),
        `expected ids: ${validate.exactTopLevelStepIds.join(", ")}; actual ids: ${actualIds.join(", ")}`
      )
    );
  }

  if (validate.topLevelStepIds && validate.topLevelStepIds.length > 0) {
    checks.push(
      check(
        "flow includes required top-level step ids",
        validate.topLevelStepIds.every((id) => actualIds.includes(id)),
        `required ids: ${validate.topLevelStepIds.join(", ")}; actual ids: ${actualIds.join(", ")}`
      )
    );
  }

  if (validate.topLevelStepOrder && validate.topLevelStepOrder.length > 0) {
    checks.push(
      check(
        "flow preserves required top-level step order",
        preservesRelativeOrder(actualIds, validate.topLevelStepOrder),
        `required order: ${validate.topLevelStepOrder.join(" -> ")}; actual ids: ${actualIds.join(" -> ")}`
      )
    );
  }

  for (const typeRequirement of validate.topLevelStepTypeCountsAtLeast ?? []) {
    const actualCount = actualTopLevelModules.filter(
      (module) => getModuleType(module) === typeRequirement.type
    ).length;
    checks.push(
      check(
        `flow includes at least ${typeRequirement.count} top-level ${typeRequirement.type} step${typeRequirement.count === 1 ? "" : "s"}`,
        actualCount >= typeRequirement.count,
        `expected at least ${typeRequirement.count}, got ${actualCount}`
      )
    );
  }

  for (const requiredStep of validate.topLevelStepTypes ?? []) {
    const module = actualTopLevelModules.find((candidate) => candidate.id === requiredStep.id);
    checks.push(check(`${requiredStep.id} step exists`, Boolean(module)));
    if (!module) {
      continue;
    }

    checks.push(
      check(
        `${requiredStep.id} type matches required`,
        getModuleType(module) === requiredStep.type,
        `expected ${requiredStep.type}, got ${getModuleType(module) ?? "(missing)"}`
      )
    );
  }

  for (const moduleRule of validate.moduleRules ?? []) {
    const module = findFlowModuleById(flow, moduleRule.id);
    checks.push(check(`${moduleRule.id} module exists for rule validation`, Boolean(module)));
    if (!module) {
      continue;
    }

    if (moduleRule.hasStopAfterIf !== undefined) {
      checks.push(
        check(
          `${moduleRule.id} stop_after_if presence matches required shape`,
          hasStopAfterIf(module) === moduleRule.hasStopAfterIf,
          `expected stop_after_if=${moduleRule.hasStopAfterIf}, got ${hasStopAfterIf(module)}`
        )
      );
    }

    if (moduleRule.hasStopAfterAllItersIf !== undefined) {
      checks.push(
        check(
          `${moduleRule.id} stop_after_all_iters_if presence matches required shape`,
          hasStopAfterAllItersIf(module) === moduleRule.hasStopAfterAllItersIf,
          `expected stop_after_all_iters_if=${moduleRule.hasStopAfterAllItersIf}, got ${hasStopAfterAllItersIf(module)}`
        )
      );
    }

    const immediateChildren = getImmediateNestedModules(module);
    const childIds = immediateChildren
      .map((child) => (typeof child.id === "string" ? child.id : null))
      .filter((id): id is string => Boolean(id));

    if (moduleRule.immediateChildStepIds && moduleRule.immediateChildStepIds.length > 0) {
      checks.push(
        check(
          `${moduleRule.id} includes required immediate child steps`,
          moduleRule.immediateChildStepIds.every((id) => childIds.includes(id)),
          `required child ids: ${moduleRule.immediateChildStepIds.join(", ")}; actual child ids: ${childIds.join(", ")}`
        )
      );
    }

    if (moduleRule.exactImmediateChildStepIds && moduleRule.exactImmediateChildStepIds.length > 0) {
      checks.push(
        check(
          `${moduleRule.id} immediate child steps match exactly`,
          stringArraysEqual(childIds, moduleRule.exactImmediateChildStepIds),
          `expected child ids: ${moduleRule.exactImmediateChildStepIds.join(", ")}; actual child ids: ${childIds.join(", ")}`
        )
      );
    }

    for (const requiredChild of moduleRule.immediateChildStepTypes ?? []) {
      const child = immediateChildren.find((candidate) => candidate.id === requiredChild.id);
      checks.push(check(`${moduleRule.id}.${requiredChild.id} child step exists`, Boolean(child)));
      if (!child) {
        continue;
      }

      checks.push(
        check(
          `${moduleRule.id}.${requiredChild.id} child type matches required`,
          getModuleType(child) === requiredChild.type,
          `expected ${requiredChild.type}, got ${getModuleType(child) ?? "(missing)"}`
        )
      );
    }

    const inputTransforms = getInputTransformRecords(module);
    for (const requiredTransform of moduleRule.requiredInputTransforms ?? []) {
      const matchedTransform = inputTransforms.find((transform) =>
        matchesRequiredInputTransform(transform, requiredTransform)
      );

      const expectedParts = [
        requiredTransform.type ? `type=${JSON.stringify(requiredTransform.type)}` : null,
        requiredTransform.expr ? `expr=${JSON.stringify(requiredTransform.expr)}` : null,
        requiredTransform.exprAnyOf && requiredTransform.exprAnyOf.length > 0
          ? `exprAnyOf=${JSON.stringify(requiredTransform.exprAnyOf)}`
          : null,
        requiredTransform.value !== undefined
          ? `value=${JSON.stringify(requiredTransform.value)}`
          : null,
      ].filter(Boolean);

      checks.push(
        check(
          `${moduleRule.id} includes required input transform (${expectedParts.join(", ")})`,
          Boolean(matchedTransform),
          `available transforms: ${summarizeInputTransforms(inputTransforms)}`
        )
      );
    }
  }

  for (const fieldRule of validate.moduleFieldRules ?? []) {
    const module = findFlowModuleById(flow, fieldRule.id);
    checks.push(check(`${fieldRule.id} module exists for field validation`, Boolean(module)));
    if (!module) {
      continue;
    }

    const actualValue = getValueAtPath(module, fieldRule.path);
    checks.push(
      check(
        `${fieldRule.id}.${fieldRule.path} matches required value`,
        valuesEqualForValidation(actualValue, fieldRule.equals),
        `expected ${JSON.stringify(fieldRule.equals)}, got ${JSON.stringify(actualValue)}`
      )
    );
  }

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

// Exact equality, including order. Use a different helper for order-insensitive checks.
function stringArraysEqual(left: string[], right: string[]): boolean {
  if (left.length !== right.length) {
    return false;
  }

  return left.every((value, index) => value === right[index]);
}

function valuesEqualForValidation(
  actual: unknown,
  expected: string | number | boolean | null
): boolean {
  if (typeof expected === "string" && typeof actual === "string") {
    return normalizeInlineExpression(actual) === normalizeInlineExpression(expected);
  }

  return actual === expected;
}

function normalizeInlineExpression(value: string): string {
  return value.replace(/\s+/g, " ").trim();
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
  return getImmediateNestedModules(module).flatMap((child) =>
    typeof child.id === "string" ? [child.id] : []
  );
}

function getImmediateNestedModules(module: Record<string, unknown>): Array<Record<string, unknown>> {
  const nested: Array<Record<string, unknown>> = [];
  const value = isObjectRecord(module.value) ? module.value : null;
  if (!value) {
    return nested;
  }

  if (Array.isArray(value.modules)) {
    nested.push(...asModuleArray(value.modules));
  }

  if (Array.isArray(value.default)) {
    nested.push(...asModuleArray(value.default));
  }

  if (Array.isArray(value.branches)) {
    for (const branch of value.branches) {
      if (!isObjectRecord(branch) || !Array.isArray(branch.modules)) {
        continue;
      }
      nested.push(...asModuleArray(branch.modules));
    }
  }

  return nested;
}

function getModuleCode(module: Record<string, unknown>): string | null {
  const value = isObjectRecord(module.value) ? module.value : null;
  return typeof value?.content === "string" ? value.content : null;
}

function getValueAtPath(record: Record<string, unknown>, dottedPath: string): unknown {
  const segments = dottedPath.split(".").filter(Boolean);
  let current: unknown = record;

  for (const segment of segments) {
    if (!isObjectRecord(current)) {
      return undefined;
    }
    current = current[segment];
  }

  return current;
}

function getInputTransformRecords(module: Record<string, unknown>): Array<Record<string, unknown>> {
  const value = isObjectRecord(module.value) ? module.value : null;
  const inputTransforms = isObjectRecord(value?.input_transforms) ? value.input_transforms : null;
  if (!inputTransforms) {
    return [];
  }

  return Object.values(inputTransforms).filter(isObjectRecord);
}

function matchesRequiredInputTransform(
  actual: Record<string, unknown>,
  required: {
    type?: string;
    expr?: string;
    exprAnyOf?: string[];
    value?: string | number | boolean | null;
  }
): boolean {
  if (required.type !== undefined && !valuesEqualForValidation(actual.type, required.type)) {
    return false;
  }

  if (required.expr !== undefined && !valuesEqualForValidation(actual.expr, required.expr)) {
    return false;
  }

  if (required.exprAnyOf !== undefined) {
    if (
      typeof actual.expr !== "string" ||
      !required.exprAnyOf.some((candidate) => valuesEqualForValidation(actual.expr, candidate))
    ) {
      return false;
    }
  }

  if (required.value !== undefined && !valuesEqualForValidation(actual.value, required.value)) {
    return false;
  }

  return true;
}

function summarizeInputTransforms(transforms: Array<Record<string, unknown>>): string {
  if (transforms.length === 0) {
    return "(none)";
  }

  return transforms
    .map((transform) =>
      JSON.stringify({
        type: transform.type,
        expr: transform.expr,
        value: transform.value,
      })
    )
    .join("; ");
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

function hasStopAfterIf(module: Record<string, unknown>): boolean {
  return isObjectRecord(module.stop_after_if);
}

function hasStopAfterAllItersIf(module: Record<string, unknown>): boolean {
  return isObjectRecord(module.stop_after_all_iters_if);
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
  return (
    fileMapsEqual(left.frontend, right.frontend) &&
    fileMapsEqual(stringifyBackend(left.backend), stringifyBackend(right.backend)) &&
    normalizeJson(canonicalizeDatatables(left.datatables ?? [])) ===
      normalizeJson(canonicalizeDatatables(right.datatables ?? []))
  );
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

function validateAppRequirements(
  app: AppFilesState,
  validate: AppValidationSpec,
  toolsUsed: string[] = []
): BenchmarkCheck[] {
  const checks: BenchmarkCheck[] = [];
  const frontendPaths = Object.keys(app.frontend ?? {});
  const backendKeys = Object.keys(app.backend ?? {});
  const datatables = app.datatables ?? [];

  if (validate.requiredFrontendPaths && validate.requiredFrontendPaths.length > 0) {
    checks.push(
      check(
        "app includes required frontend paths",
        validate.requiredFrontendPaths.every((filePath) => frontendPaths.includes(filePath)),
        `required paths: ${validate.requiredFrontendPaths.join(", ")}; actual paths: ${frontendPaths.join(", ")}`
      )
    );
  }

  for (const contentRequirement of validate.requiredFrontendFileContent ?? []) {
    const content = app.frontend?.[contentRequirement.path];
    checks.push(
      check(
        `frontend ${contentRequirement.path} exists for content validation`,
        content !== undefined
      )
    );
    if (content === undefined) {
      continue;
    }

    checks.push(
      check(
        `frontend ${contentRequirement.path} includes required content`,
        contentIncludesAll(content, contentRequirement.includes),
        missingSnippetsDetails(content, contentRequirement.includes)
      )
    );
  }

  if (validate.requiredBackendRunnableKeys && validate.requiredBackendRunnableKeys.length > 0) {
    checks.push(
      check(
        "app includes required backend runnables",
        validate.requiredBackendRunnableKeys.every((key) => backendKeys.includes(key)),
        `required runnables: ${validate.requiredBackendRunnableKeys.join(", ")}; actual runnables: ${backendKeys.join(", ")}`
      )
    );
  }

  if (typeof validate.backendRunnableCountAtLeast === "number") {
    checks.push(
      check(
        `app includes at least ${validate.backendRunnableCountAtLeast} backend runnable${validate.backendRunnableCountAtLeast === 1 ? "" : "s"}`,
        backendKeys.length >= validate.backendRunnableCountAtLeast,
        `expected at least ${validate.backendRunnableCountAtLeast}, got ${backendKeys.length}`
      )
    );
  }

  for (const runnableRequirement of validate.requiredBackendRunnableTypes ?? []) {
    const runnable = app.backend?.[runnableRequirement.key];
    checks.push(check(`${runnableRequirement.key} backend runnable exists`, Boolean(runnable)));
    if (!runnable) {
      continue;
    }

    checks.push(
      check(
        `${runnableRequirement.key} backend runnable type matches required`,
        runnable.type === runnableRequirement.type,
        `expected ${runnableRequirement.type}, got ${runnable.type ?? "(missing)"}`
      )
    );
  }

  for (const contentRequirement of validate.requiredBackendRunnableContent ?? []) {
    const runnable = app.backend?.[contentRequirement.key];
    checks.push(
      check(
        `${contentRequirement.key} backend runnable exists for content validation`,
        Boolean(runnable)
      )
    );
    if (!runnable) {
      continue;
    }

    const content = runnable.inlineScript?.content ?? "";
    checks.push(
      check(
        `${contentRequirement.key} backend runnable includes required content`,
        contentIncludesAll(content, contentRequirement.includes),
        missingSnippetsDetails(content, contentRequirement.includes)
      )
    );
  }

  if (typeof validate.datatableCountAtLeast === "number") {
    checks.push(
      check(
        `app includes at least ${validate.datatableCountAtLeast} datatable${validate.datatableCountAtLeast === 1 ? "" : "s"}`,
        datatables.length >= validate.datatableCountAtLeast,
        `expected at least ${validate.datatableCountAtLeast}, got ${datatables.length}`
      )
    );
  }

  if (typeof validate.datatableTableCountAtLeast === "number") {
    const actualTableCount = countDatatableTables(datatables);
    checks.push(
      check(
        `app includes at least ${validate.datatableTableCountAtLeast} datatable table${validate.datatableTableCountAtLeast === 1 ? "" : "s"}`,
        actualTableCount >= validate.datatableTableCountAtLeast,
        `expected at least ${validate.datatableTableCountAtLeast}, got ${actualTableCount}`
      )
    );
  }

  if (typeof validate.datatableTableCountExactly === "number") {
    const actualTableCount = countDatatableTables(datatables);
    checks.push(
      check(
        `app includes exactly ${validate.datatableTableCountExactly} datatable table${validate.datatableTableCountExactly === 1 ? "" : "s"}`,
        actualTableCount === validate.datatableTableCountExactly,
        `expected exactly ${validate.datatableTableCountExactly}, got ${actualTableCount}`
      )
    );
  }

  for (const datatableRequirement of validate.requiredDatatables ?? []) {
    const label = datatableRequirement.datatableName
      ? `${datatableRequirement.datatableName}/${datatableRequirement.schema}.${datatableRequirement.table}`
      : `${datatableRequirement.schema}.${datatableRequirement.table}`;
    checks.push(
      check(
        `datatable table ${label} exists`,
        hasDatatableTable(datatables, datatableRequirement)
      )
    );
  }

  for (const toolName of validate.requiredToolsUsed ?? []) {
    checks.push(
      check(
        `tool ${toolName} was used`,
        toolsUsed.includes(toolName),
        `tools used: ${toolsUsed.join(", ") || "none"}`
      )
    );
  }

  for (const forbiddenSnippet of validate.forbiddenAppContent ?? []) {
    checks.push(
      check(
        `app does not include forbidden content '${forbiddenSnippet}'`,
        !appContentIncludes(app, forbiddenSnippet),
        `forbidden snippet: ${forbiddenSnippet}`
      )
    );
  }

  return checks;
}

function contentIncludesAll(content: string, snippets: string[]): boolean {
  const normalizedContent = content.toLowerCase();
  return snippets.every((snippet) => normalizedContent.includes(snippet.toLowerCase()));
}

function missingSnippetsDetails(content: string, snippets: string[]): string {
  const normalizedContent = content.toLowerCase();
  const missing = snippets.filter(
    (snippet) => !normalizedContent.includes(snippet.toLowerCase())
  );
  return missing.length > 0 ? `missing snippets: ${missing.join(", ")}` : "";
}

function appContentIncludes(app: AppFilesState, snippet: string): boolean {
  const normalizedSnippet = snippet.toLowerCase();
  const frontendContent = Object.values(app.frontend ?? {});
  const backendContent = Object.values(app.backend ?? {}).map(
    (runnable) => runnable.inlineScript?.content ?? ""
  );
  return [...frontendContent, ...backendContent].some((content) =>
    content.toLowerCase().includes(normalizedSnippet)
  );
}

function countDatatableTables(datatables: AppDatatableState[]): number {
  return datatables.reduce((count, datatable) => {
    return (
      count +
      Object.values(datatable.schemas ?? {}).reduce((schemaCount, tables) => {
        return schemaCount + Object.keys(tables ?? {}).length;
      }, 0)
    );
  }, 0);
}

function hasDatatableTable(
  datatables: AppDatatableState[],
  input: { schema: string; table: string; datatableName?: string }
): boolean {
  return datatables.some((datatable) => {
    if (input.datatableName && datatable.datatable_name !== input.datatableName) {
      return false;
    }
    return Boolean(datatable.schemas?.[input.schema]?.[input.table]);
  });
}

function canonicalizeDatatables(datatables: AppDatatableState[]): AppDatatableState[] {
  return [...datatables]
    .map((datatable) => ({
      datatable_name: datatable.datatable_name,
      error: datatable.error,
      schemas: Object.fromEntries(
        Object.entries(datatable.schemas ?? {})
          .sort(([left], [right]) => left.localeCompare(right))
          .map(([schemaName, tables]) => [
            schemaName,
            Object.fromEntries(
              Object.entries(tables ?? {})
                .sort(([left], [right]) => left.localeCompare(right))
                .map(([tableName, columns]) => [
                  tableName,
                  Object.fromEntries(
                    Object.entries(columns ?? {}).sort(([left], [right]) => left.localeCompare(right))
                  ),
                ])
            ),
          ])
      ),
    }))
    .sort((left, right) => left.datatable_name.localeCompare(right.datatable_name));
}
