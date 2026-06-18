import path from "node:path";
import ts from "typescript";
import type { LintResult } from "../../frontend/src/lib/components/copilot/chat/app/core";

const FRONTEND_ROOT = "/__ai_evals__/frontend";
const BACKEND_ROOT = "/__ai_evals__/backend";
const FRONTEND_REACT_SHIM_PATH = `${FRONTEND_ROOT}/__react_shim__.d.ts`;
const FRONTEND_WMILL_TYPES_PATH = `${FRONTEND_ROOT}/wmill.d.ts`;
const BACKEND_WINDMILL_CLIENT_SHIM_PATH = `${BACKEND_ROOT}/__windmill_client__.d.ts`;
const TS_LIKE_LANGUAGES = new Set([
  "bun",
  "deno",
  "nativets",
  "bunnative",
  "ts",
  "typescript",
]);
const JS_LIKE_LANGUAGES = new Set(["javascript", "js", "nodejs"]);
const SAFE_TYPE_REFERENCE_NAMES = new Set([
  "Array",
  "Date",
  "Exclude",
  "Extract",
  "NonNullable",
  "Omit",
  "Partial",
  "Pick",
  "Promise",
  "Readonly",
  "ReadonlyArray",
  "Record",
  "Required",
  "ReturnType",
  "Uppercase",
  "Lowercase",
  "Capitalize",
  "Uncapitalize",
]);

const FRONTEND_REACT_SHIM = `declare namespace React {
  type SetStateAction<S> = S | ((prevState: S) => S);
  type Dispatch<A> = (value: A) => void;
  type FC<P = {}> = (props: P) => any;
  type ReactNode = any;
  interface FormEvent<T = EventTarget> {
    preventDefault(): void;
    target: T;
    currentTarget: T;
  }
  interface ChangeEvent<T = EventTarget> {
    target: T;
    currentTarget: T;
  }
}

declare namespace JSX {
  interface IntrinsicAttributes {
    key?: any;
  }
  interface IntrinsicElements {
    [elementName: string]: any;
  }
}

declare module "react" {
  export type SetStateAction<S> = React.SetStateAction<S>;
  export type Dispatch<A> = React.Dispatch<A>;
  export type FC<P = {}> = React.FC<P>;
  export type ReactNode = React.ReactNode;
  export type FormEvent<T = EventTarget> = React.FormEvent<T>;
  export type ChangeEvent<T = EventTarget> = React.ChangeEvent<T>;
  export function useState<S>(initialState: S | (() => S)): [S, Dispatch<SetStateAction<S>>];
  export function useEffect(effect: () => void | (() => void), deps?: readonly unknown[]): void;
  const React: any;
  export default React;
}
`;

const BACKEND_WINDMILL_CLIENT_SHIM = `declare const console: {
  log: (...args: any[]) => void;
  error: (...args: any[]) => void;
  warn: (...args: any[]) => void;
};

declare module "windmill-client" {
  interface SqlQueryResult {
    fetch(): Promise<any>;
    fetchOne(): Promise<any>;
  }

  interface SqlTemplateFunction {
    (strings: TemplateStringsArray, ...values: any[]): SqlQueryResult;
  }

  interface WindmillClient {
    datatable(name?: string): SqlTemplateFunction;
    ducklake(name?: string): SqlTemplateFunction;
    [key: string]: any;
  }

  const wmill: WindmillClient;
  export = wmill;
}
`;

export interface AppDiagnosticRunnable {
  name?: string;
  type?: string;
  path?: string;
  inlineScript?: {
    language?: string;
    content?: string;
  };
}

export interface AppStaticDiagnostic {
  source: "frontend" | "backend";
  target: string;
  message: string;
  line?: number;
  column?: number;
  code?: number;
}

export interface AppDiagnosticsResult {
  lintResult: LintResult;
  diagnostics: AppStaticDiagnostic[];
}

export function buildAppWmillTypes(
  backend: Record<string, AppDiagnosticRunnable> = {},
): string {
  return `// THIS FILE IS READ-ONLY
// AND GENERATED AUTOMATICALLY FROM YOUR RUNNABLES

export declare const backend: {
${Object.entries(backend)
  .map(
    ([key, runnable]) =>
      `  ${JSON.stringify(key)}: ${getRunnableSignature(runnable, false)};`,
  )
  .join("\n")}
};

export declare const backendAsync: {
${Object.entries(backend)
  .map(
    ([key, runnable]) =>
      `  ${JSON.stringify(key)}: ${getRunnableSignature(runnable, true)};`,
  )
  .join("\n")}
};

export type Job = {
  type: "QueuedJob" | "CompletedJob";
  id: string;
  created_at: number;
  started_at: number | undefined;
  duration_ms: number;
  success: boolean;
  args: any;
  result: any;
};

export declare function waitJob(id: string): Promise<Job>;
export declare function getJob(id: string): Promise<Job>;

export type StreamUpdate = {
  new_result_stream?: string;
  stream_offset?: number;
};

export declare function streamJob(id: string, onUpdate?: (data: StreamUpdate) => void): Promise<any>;
`;
}

export function collectAppDiagnostics(input: {
  frontend: Record<string, string>;
  backend: Record<string, AppDiagnosticRunnable>;
}): AppDiagnosticsResult {
  const frontendDiagnostics = collectFrontendDiagnostics(
    input.frontend,
    input.backend,
  );
  const backendDiagnostics = collectBackendDiagnostics(input.backend);
  const diagnostics = dedupeDiagnostics([
    ...frontendDiagnostics,
    ...backendDiagnostics,
  ]).sort(compareDiagnostics);

  return {
    diagnostics,
    lintResult: {
      errors: {
        frontend: groupMessages(
          diagnostics.filter((diagnostic) => diagnostic.source === "frontend"),
        ),
        backend: groupMessages(
          diagnostics.filter((diagnostic) => diagnostic.source === "backend"),
        ),
      },
      warnings: {
        frontend: {},
        backend: {},
      },
      errorCount: diagnostics.length,
      warningCount: 0,
    },
  };
}

function collectFrontendDiagnostics(
  frontend: Record<string, string>,
  backend: Record<string, AppDiagnosticRunnable>,
): AppStaticDiagnostic[] {
  const frontendFiles = Object.entries(frontend)
    .filter(([filePath]) => isFrontendCodeFile(filePath))
    .map(
      ([filePath, content]) =>
        [toFrontendVirtualPath(filePath), content] as const,
    );

  const virtualFiles = new Map<string, string>([
    [FRONTEND_REACT_SHIM_PATH, FRONTEND_REACT_SHIM],
    [
      FRONTEND_WMILL_TYPES_PATH,
      wrapModuleDeclaration("wmill", buildAppWmillTypes(backend)),
    ],
    ...frontendFiles,
  ]);
  const host = createVirtualCompilerHost(
    virtualFiles,
    getFrontendCompilerOptions(),
  );
  const rootNames = [...virtualFiles.keys()];
  const program = ts.createProgram({
    rootNames,
    options: getFrontendCompilerOptions(),
    host,
  });

  return ts.getPreEmitDiagnostics(program).flatMap((diagnostic) =>
    mapTypeScriptDiagnostic({
      diagnostic,
      source: "frontend",
      toTarget(fileName) {
        const normalized = normalizeFileName(fileName);
        if (normalized === FRONTEND_WMILL_TYPES_PATH) {
          return "/wmill.d.ts";
        }
        if (!normalized.startsWith(`${FRONTEND_ROOT}/`)) {
          return null;
        }
        if (normalized === FRONTEND_REACT_SHIM_PATH) {
          return null;
        }
        return normalized.slice(FRONTEND_ROOT.length);
      },
    }),
  );
}

function collectBackendDiagnostics(
  backend: Record<string, AppDiagnosticRunnable>,
): AppStaticDiagnostic[] {
  const backendFiles = Object.entries(backend)
    .filter(([, runnable]) => isTypeCheckableBackendRunnable(runnable))
    .map(
      ([key, runnable]) =>
        [
          `${BACKEND_ROOT}/${key}/main.${getBackendFileExtension(runnable.inlineScript?.language)}`,
          runnable.inlineScript?.content ?? "",
        ] as const,
    );

  if (backendFiles.length === 0) {
    return [];
  }

  const virtualFiles = new Map<string, string>([
    [BACKEND_WINDMILL_CLIENT_SHIM_PATH, BACKEND_WINDMILL_CLIENT_SHIM],
    ...backendFiles,
  ]);
  const host = createVirtualCompilerHost(
    virtualFiles,
    getBackendCompilerOptions(),
  );
  const rootNames = [...virtualFiles.keys()];
  const program = ts.createProgram({
    rootNames,
    options: getBackendCompilerOptions(),
    host,
  });

  return ts.getPreEmitDiagnostics(program).flatMap((diagnostic) =>
    mapTypeScriptDiagnostic({
      diagnostic,
      source: "backend",
      toTarget(fileName) {
        const normalized = normalizeFileName(fileName);
        if (normalized === BACKEND_WINDMILL_CLIENT_SHIM_PATH) {
          return null;
        }
        if (!normalized.startsWith(`${BACKEND_ROOT}/`)) {
          return null;
        }
        const relativePath = normalized.slice(BACKEND_ROOT.length + 1);
        const runnableKey = relativePath.split("/")[0];
        return runnableKey || null;
      },
    }),
  );
}

function getFrontendCompilerOptions(): ts.CompilerOptions {
  return {
    allowJs: true,
    checkJs: true,
    esModuleInterop: true,
    allowSyntheticDefaultImports: true,
    jsx: ts.JsxEmit.Preserve,
    module: ts.ModuleKind.ESNext,
    moduleResolution: ts.ModuleResolutionKind.Node10,
    noEmit: true,
    noImplicitAny: false,
    skipLibCheck: true,
    strict: false,
    target: ts.ScriptTarget.ES2022,
    lib: ["lib.es2022.d.ts", "lib.dom.d.ts"],
  };
}

function getBackendCompilerOptions(): ts.CompilerOptions {
  return {
    allowJs: true,
    checkJs: true,
    esModuleInterop: true,
    allowSyntheticDefaultImports: true,
    module: ts.ModuleKind.ESNext,
    moduleResolution: ts.ModuleResolutionKind.Node10,
    noEmit: true,
    noImplicitAny: false,
    skipLibCheck: true,
    strict: false,
    target: ts.ScriptTarget.ES2022,
    lib: ["lib.es2022.d.ts"],
  };
}

function createVirtualCompilerHost(
  files: Map<string, string>,
  options: ts.CompilerOptions,
): ts.CompilerHost {
  const originalHost = ts.createCompilerHost(options, true);
  const originalGetSourceFile = originalHost.getSourceFile.bind(originalHost);
  const originalReadFile = originalHost.readFile.bind(originalHost);
  const originalFileExists = originalHost.fileExists.bind(originalHost);
  const originalDirectoryExists =
    originalHost.directoryExists?.bind(originalHost);
  const originalGetDirectories =
    originalHost.getDirectories?.bind(originalHost);

  return {
    ...originalHost,
    getCurrentDirectory: () => "/",
    getSourceFile(
      fileName,
      languageVersion,
      onError,
      shouldCreateNewSourceFile,
    ) {
      const normalized = normalizeFileName(fileName);
      const content = files.get(normalized);
      if (content !== undefined) {
        return ts.createSourceFile(fileName, content, languageVersion, true);
      }
      return originalGetSourceFile(
        fileName,
        languageVersion,
        onError,
        shouldCreateNewSourceFile,
      );
    },
    readFile(fileName) {
      const normalized = normalizeFileName(fileName);
      return files.get(normalized) ?? originalReadFile(fileName);
    },
    fileExists(fileName) {
      const normalized = normalizeFileName(fileName);
      return files.has(normalized) || originalFileExists(fileName);
    },
    directoryExists(dirName) {
      const normalized = normalizeFileName(dirName);
      return (
        hasVirtualDirectory(files, normalized) ||
        originalDirectoryExists?.(dirName) ||
        false
      );
    },
    getDirectories(dirName) {
      const normalized = normalizeFileName(dirName);
      const virtualDirectories = listVirtualDirectories(files, normalized);
      const diskDirectories = originalGetDirectories?.(dirName) ?? [];
      return [...new Set([...diskDirectories, ...virtualDirectories])];
    },
    realpath(fileName) {
      return normalizeFileName(fileName);
    },
    writeFile() {},
  };
}

function mapTypeScriptDiagnostic(input: {
  diagnostic: ts.Diagnostic;
  source: "frontend" | "backend";
  toTarget: (fileName: string) => string | null;
}): AppStaticDiagnostic[] {
  const { diagnostic, source, toTarget } = input;
  if (!diagnostic.file) {
    return [];
  }

  const target = toTarget(diagnostic.file.fileName);
  if (!target) {
    return [];
  }

  const position =
    diagnostic.start === undefined
      ? undefined
      : diagnostic.file.getLineAndCharacterOfPosition(diagnostic.start);

  return [
    {
      source,
      target,
      message: ts
        .flattenDiagnosticMessageText(diagnostic.messageText, "\n")
        .trim(),
      line: position ? position.line + 1 : undefined,
      column: position ? position.character + 1 : undefined,
      code: diagnostic.code,
    },
  ];
}

function groupMessages(
  diagnostics: AppStaticDiagnostic[],
): Record<string, string[]> {
  const grouped: Record<string, string[]> = {};

  for (const diagnostic of diagnostics) {
    grouped[diagnostic.target] ??= [];
    grouped[diagnostic.target].push(formatLintMessage(diagnostic));
  }

  return grouped;
}

function formatLintMessage(diagnostic: AppStaticDiagnostic): string {
  if (diagnostic.line !== undefined) {
    return `Line ${diagnostic.line}: ${diagnostic.message}`;
  }
  return diagnostic.message;
}

function dedupeDiagnostics(
  diagnostics: AppStaticDiagnostic[],
): AppStaticDiagnostic[] {
  const uniqueDiagnostics = new Map<string, AppStaticDiagnostic>();

  for (const diagnostic of diagnostics) {
    const key = [
      diagnostic.source,
      diagnostic.target,
      diagnostic.code ?? "",
      diagnostic.line ?? "",
      diagnostic.column ?? "",
      diagnostic.message,
    ].join("::");
    if (!uniqueDiagnostics.has(key)) {
      uniqueDiagnostics.set(key, diagnostic);
    }
  }

  return [...uniqueDiagnostics.values()];
}

function compareDiagnostics(
  a: AppStaticDiagnostic,
  b: AppStaticDiagnostic,
): number {
  if (a.source !== b.source) {
    return a.source.localeCompare(b.source);
  }
  if (a.target !== b.target) {
    return a.target.localeCompare(b.target);
  }
  if ((a.line ?? 0) !== (b.line ?? 0)) {
    return (a.line ?? 0) - (b.line ?? 0);
  }
  if ((a.column ?? 0) !== (b.column ?? 0)) {
    return (a.column ?? 0) - (b.column ?? 0);
  }
  return a.message.localeCompare(b.message);
}

function toFrontendVirtualPath(filePath: string): string {
  const normalizedPath = normalizeAppFilePath(filePath);
  return `${FRONTEND_ROOT}${normalizedPath}`;
}

function normalizeAppFilePath(filePath: string): string {
  const normalizedPath = normalizeFileName(filePath);
  return normalizedPath.startsWith("/") ? normalizedPath : `/${normalizedPath}`;
}

function normalizeFileName(fileName: string): string {
  return path.posix.normalize(fileName.replace(/\\/g, "/"));
}

function hasVirtualDirectory(
  files: Map<string, string>,
  dirName: string,
): boolean {
  const normalizedDirectory = dirName.endsWith("/") ? dirName : `${dirName}/`;
  for (const fileName of files.keys()) {
    if (fileName === dirName || fileName.startsWith(normalizedDirectory)) {
      return true;
    }
  }
  return false;
}

function listVirtualDirectories(
  files: Map<string, string>,
  dirName: string,
): string[] {
  const normalizedDirectory = dirName.endsWith("/") ? dirName : `${dirName}/`;
  const directories = new Set<string>();

  for (const fileName of files.keys()) {
    if (!fileName.startsWith(normalizedDirectory)) {
      continue;
    }
    const relativePath = fileName.slice(normalizedDirectory.length);
    const [segment] = relativePath.split("/");
    if (segment && relativePath.includes("/")) {
      directories.add(path.posix.join(dirName, segment));
    }
  }

  return [...directories];
}

function wrapModuleDeclaration(moduleName: string, content: string): string {
  const indentedContent = content
    .trim()
    .split("\n")
    .map((line) => `  ${line}`)
    .join("\n");

  return `declare module "${moduleName}" {\n${indentedContent}\n}\n`;
}

function getRunnableSignature(
  runnable: AppDiagnosticRunnable | undefined,
  asyncMode: boolean,
): string {
  const returnType = asyncMode ? "Promise<string>" : "Promise<any>";
  const parameter = getRunnableParameterSignature(runnable);
  return `${parameter} => ${returnType}`;
}

function getRunnableParameterSignature(
  runnable: AppDiagnosticRunnable | undefined,
): string {
  const parameterInfo = getRunnableParameterInfo(runnable);
  if (!parameterInfo) {
    return "()";
  }

  const parameterType = parameterInfo.typeText ?? "any";
  if (parameterInfo.optional) {
    return `(args?: ${parameterType})`;
  }
  return `(args: ${parameterType})`;
}

function getRunnableParameterInfo(
  runnable: AppDiagnosticRunnable | undefined,
): { typeText?: string; optional: boolean } | null {
  if (
    !runnable?.inlineScript?.content ||
    !isTypeCheckableBackendRunnable(runnable)
  ) {
    return { typeText: "any", optional: true };
  }

  const sourceFile = ts.createSourceFile(
    "main.ts",
    runnable.inlineScript.content,
    ts.ScriptTarget.Latest,
    true,
    getScriptKindForLanguage(runnable.inlineScript.language),
  );
  const mainDeclaration = findExportedMainDeclaration(sourceFile);

  if (!mainDeclaration || mainDeclaration.parameters.length === 0) {
    return null;
  }

  const [parameter] = mainDeclaration.parameters;
  const optional =
    Boolean(parameter.questionToken) || Boolean(parameter.initializer);

  if (!parameter.type || !isPortableTypeNode(parameter.type)) {
    return { typeText: "any", optional: true };
  }

  return {
    typeText: parameter.type.getText(sourceFile).trim(),
    optional,
  };
}

function findExportedMainDeclaration(
  sourceFile: ts.SourceFile,
): ts.SignatureDeclarationBase | null {
  for (const statement of sourceFile.statements) {
    if (
      ts.isFunctionDeclaration(statement) &&
      statement.name?.text === "main" &&
      hasExportModifier(statement)
    ) {
      return statement;
    }

    if (!ts.isVariableStatement(statement) || !hasExportModifier(statement)) {
      continue;
    }

    for (const declaration of statement.declarationList.declarations) {
      if (
        !ts.isIdentifier(declaration.name) ||
        declaration.name.text !== "main"
      ) {
        continue;
      }
      const initializer = declaration.initializer;
      if (
        initializer &&
        (ts.isArrowFunction(initializer) ||
          ts.isFunctionExpression(initializer))
      ) {
        return initializer;
      }
    }
  }

  return null;
}

function hasExportModifier(node: ts.Node): boolean {
  const modifiers = ts.canHaveModifiers(node)
    ? ts.getModifiers(node)
    : undefined;
  return Boolean(
    modifiers?.some(
      (modifier) => modifier.kind === ts.SyntaxKind.ExportKeyword,
    ),
  );
}

function isPortableTypeNode(node: ts.TypeNode): boolean {
  if (
    isKeywordTypeNode(node) ||
    ts.isArrayTypeNode(node) ||
    ts.isTupleTypeNode(node) ||
    ts.isLiteralTypeNode(node) ||
    ts.isTypeLiteralNode(node)
  ) {
    return true;
  }

  if (ts.isParenthesizedTypeNode(node) || ts.isTypeOperatorNode(node)) {
    return isPortableTypeNode(node.type);
  }

  if (ts.isUnionTypeNode(node) || ts.isIntersectionTypeNode(node)) {
    return node.types.every((typeNode) => isPortableTypeNode(typeNode));
  }

  if (ts.isTypeReferenceNode(node)) {
    if (
      !ts.isIdentifier(node.typeName) ||
      !SAFE_TYPE_REFERENCE_NAMES.has(node.typeName.text)
    ) {
      return false;
    }
    return (node.typeArguments ?? []).every((typeArgument) =>
      isPortableTypeNode(typeArgument),
    );
  }

  return false;
}

function isKeywordTypeNode(node: ts.TypeNode): boolean {
  switch (node.kind) {
    case ts.SyntaxKind.AnyKeyword:
    case ts.SyntaxKind.BigIntKeyword:
    case ts.SyntaxKind.BooleanKeyword:
    case ts.SyntaxKind.NeverKeyword:
    case ts.SyntaxKind.NumberKeyword:
    case ts.SyntaxKind.ObjectKeyword:
    case ts.SyntaxKind.StringKeyword:
    case ts.SyntaxKind.SymbolKeyword:
    case ts.SyntaxKind.UndefinedKeyword:
    case ts.SyntaxKind.UnknownKeyword:
    case ts.SyntaxKind.VoidKeyword:
      return true;
    default:
      return false;
  }
}

function isFrontendCodeFile(filePath: string): boolean {
  const extension = path.posix.extname(filePath).toLowerCase();
  return (
    extension === ".js" ||
    extension === ".jsx" ||
    extension === ".ts" ||
    extension === ".tsx"
  );
}

function isTypeCheckableBackendRunnable(
  runnable: AppDiagnosticRunnable | undefined,
): boolean {
  if (!runnable || runnable.type !== "inline") {
    return false;
  }
  const language = runnable.inlineScript?.language?.toLowerCase() ?? "";
  return TS_LIKE_LANGUAGES.has(language) || JS_LIKE_LANGUAGES.has(language);
}

function getBackendFileExtension(language: string | undefined): string {
  const normalizedLanguage = language?.toLowerCase() ?? "";
  return JS_LIKE_LANGUAGES.has(normalizedLanguage) ? "js" : "ts";
}

function getScriptKindForLanguage(language: string | undefined): ts.ScriptKind {
  const normalizedLanguage = language?.toLowerCase() ?? "";
  return JS_LIKE_LANGUAGES.has(normalizedLanguage)
    ? ts.ScriptKind.JS
    : ts.ScriptKind.TS;
}
