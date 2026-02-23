import { stat, readdir } from "node:fs/promises";
import process from "node:process";

import { colors } from "@cliffy/ansi/colors";
import { Command } from "@cliffy/command";
import * as log from "../../core/log.ts";
import * as path from "node:path";
import { sep as SEP } from "node:path";
import { yamlParseFile } from "../../utils/yaml.ts";
import { GlobalOptions } from "../../types.ts";
import { mergeConfigWithConfigFile } from "../../core/conf.ts";
import {
  FSFSElement,
  ignoreF,
  readDirRecursiveWithIgnore,
} from "../sync/sync.ts";
import {
  getValidationTargetFromFilename,
  type ValidationTarget,
  WindmillYamlValidator,
} from "windmill-yaml-validator";
import {
  inferContentTypeFromFilePath,
  languageNeedsLock,
  ScriptLanguage,
} from "../../utils/script_common.ts";
import {
  isFlowInlineScriptPath,
  isAppInlineScriptPath,
  isRawAppPath,
  getFolderSuffix,
} from "../../utils/resource_folders.ts";
import { exts } from "../script/script.ts";

interface LintOptions extends GlobalOptions {
  json?: boolean;
  failOnWarn?: boolean;
  locksRequired?: boolean;
}

interface FileIssue {
  path: string;
  target: string;
  errors: string[];
}

interface LintWarning {
  path: string;
  message: string;
}

export interface LintReport {
  scannedFiles: number;
  validatedFiles: number;
  validFiles: number;
  invalidFiles: number;
  skippedUnsupportedFiles: number;
  warnings: LintWarning[];
  issues: FileIssue[];
  success: boolean;
  exitCode: number;
}

const YAML_FILE_REGEX = /\.ya?ml$/i;
const NATIVE_TRIGGER_REGEX = /\.[^.]+_native_trigger\.ya?ml$/i;

function normalizePath(p: string): string {
  return p.replaceAll(SEP, "/");
}

function isUnsupportedTriggerPath(filePath: string): boolean {
  return NATIVE_TRIGGER_REGEX.test(filePath);
}

function formatTarget(target: ValidationTarget): string {
  if (target.type === "trigger") {
    return `${target.triggerKind}_trigger`;
  }
  return target.type;
}

export function formatValidationError(error: {
  instancePath?: string;
  keyword?: string;
  message?: string;
  params?: {
    missingProperty?: string;
    allowedValues?: unknown[];
    additionalProperty?: string;
  };
}): string {
  const instancePath = error.instancePath && error.instancePath.length > 0
    ? error.instancePath
    : "/";
  if (error.keyword === "required" && error.params?.missingProperty) {
    return `${instancePath} missing required property '${error.params.missingProperty}'`;
  }
  if (
    error.keyword === "additionalProperties" &&
    error.params?.additionalProperty
  ) {
    return `${instancePath} has unknown property '${error.params.additionalProperty}'`;
  }
  if (error.keyword === "enum" && error.params?.allowedValues) {
    const allowed = error.params.allowedValues
      .filter((v) => v !== null)
      .map((v) => `'${v}'`)
      .join(", ");
    return `${instancePath} must be one of: ${allowed}`;
  }
  if (error.message) {
    return `${instancePath} ${error.message}`;
  }
  return `${instancePath} validation error`;
}

function formatYamlDiagnostics(parsed: { diagnostics?: Array<{ message?: string }> }): string[] {
  const diagnostics = parsed?.diagnostics;
  if (!Array.isArray(diagnostics) || diagnostics.length === 0) {
    return [];
  }
  return diagnostics.map((d) => d?.message || "Invalid YAML document");
}

/**
 * Check if a lock value represents an actually resolved lock.
 * Returns true if the lock is present and valid, false if missing.
 * For `!inline` references, checks that the referenced file exists and is non-empty.
 */
async function isLockResolved(
  lockValue: string | string[] | undefined,
  baseDir: string,
): Promise<boolean> {
  if (lockValue === undefined) return false;

  // Array lock (v2 format) - if non-empty, locks are present
  if (Array.isArray(lockValue)) {
    const joined = lockValue.join("\n");
    if (joined === "") return false;
    if (joined.startsWith("!inline ")) {
      return await checkInlineFile(joined.substring("!inline ".length), baseDir);
    }
    return true;
  }

  if (lockValue === "") return false;

  // Inline file reference
  if (lockValue.startsWith("!inline ")) {
    return await checkInlineFile(lockValue.substring("!inline ".length), baseDir);
  }

  // Embedded lock content
  return true;
}

async function checkInlineFile(
  relativePath: string,
  baseDir: string,
): Promise<boolean> {
  const fullPath = path.join(baseDir, relativePath.trim());
  try {
    const s = await stat(fullPath);
    return s.size > 0;
  } catch {
    return false;
  }
}

/**
 * Recursively find rawscript modules in a flow's module tree.
 */
function findRawScriptsInModules(
  modules: any[],
): { language: string; lock: any; id: string }[] {
  const results: { language: string; lock: any; id: string }[] = [];
  if (!modules || !Array.isArray(modules)) return results;

  for (const m of modules) {
    if (!m?.value?.type) continue;

    if (m.value.type === "rawscript") {
      results.push({
        language: m.value.language,
        lock: m.value.lock,
        id: m.id ?? "unknown",
      });
    } else if (
      m.value.type === "forloopflow" ||
      m.value.type === "whileloopflow"
    ) {
      results.push(...findRawScriptsInModules(m.value.modules));
    } else if (m.value.type === "branchall") {
      for (const b of m.value.branches ?? []) {
        results.push(...findRawScriptsInModules(b.modules));
      }
    } else if (m.value.type === "branchone") {
      for (const b of m.value.branches ?? []) {
        results.push(...findRawScriptsInModules(b.modules));
      }
      if (m.value.default) {
        results.push(...findRawScriptsInModules(m.value.default));
      }
    } else if (m.value.type === "aiagent") {
      for (const tool of m.value.tools ?? []) {
        const toolValue = tool.value;
        if (
          toolValue?.tool_type === "flowmodule" &&
          toolValue?.type === "rawscript"
        ) {
          results.push({
            language: toolValue.language,
            lock: toolValue.lock,
            id: tool.id ?? "unknown",
          });
        }
      }
    }
  }

  return results;
}

/**
 * Recursively find inlineScript objects in a normal app's value structure.
 * Follows the same traversal as traverseAndProcessInlineScripts in app_metadata.ts.
 */
function findInlineScriptsInApp(
  obj: any,
  currentPath: string[] = [],
): { language: string; lock: any; path: string }[] {
  const results: { language: string; lock: any; path: string }[] = [];
  if (!obj || typeof obj !== "object") return results;

  if (Array.isArray(obj)) {
    for (let i = 0; i < obj.length; i++) {
      results.push(
        ...findInlineScriptsInApp(obj[i], [...currentPath, `[${i}]`]),
      );
    }
    return results;
  }

  for (const [key, value] of Object.entries(obj)) {
    if (key === "inlineScript" && typeof value === "object" && value !== null) {
      const script = value as Record<string, any>;
      if (script.language) {
        results.push({
          language: script.language,
          lock: script.lock,
          path: [...currentPath, key].join("."),
        });
      }
    } else {
      results.push(
        ...findInlineScriptsInApp(value, [...currentPath, key]),
      );
    }
  }

  return results;
}

/**
 * Check raw app backend runnables for missing locks.
 * Reads YAML config files and code files from the backend/ folder.
 */
async function checkRawAppRunnables(
  backendDir: string,
  rawAppYamlPath: string,
  defaultTs: "bun" | "deno" | undefined,
): Promise<FileIssue[]> {
  const issues: FileIssue[] = [];

  const allFiles: string[] = [];
  const entries = await readdir(backendDir, { withFileTypes: true });
  for (const entry of entries) {
    if (entry.isFile()) {
      allFiles.push(entry.name);
    }
  }

  // Track processed IDs to avoid duplicates
  const processedIds = new Set<string>();

  // Process YAML files (explicit config)
  for (const fileName of allFiles) {
    if (!fileName.endsWith(".yaml")) continue;

    const runnableId = fileName.replace(".yaml", "");
    processedIds.add(runnableId);

    const filePath = path.join(backendDir, fileName);
    let runnable: Record<string, any>;
    try {
      runnable = (await yamlParseFile(filePath)) as Record<string, any>;
    } catch {
      continue;
    }

    // Only inline runnables need lock checking
    if (runnable?.type !== "inline") continue;

    // Find the content file to determine language
    let language: string | null = null;
    for (const codeFile of allFiles) {
      if (
        codeFile.endsWith(".yaml") || codeFile.endsWith(".lock") ||
        !codeFile.startsWith(runnableId + ".")
      ) continue;
      language = inferContentTypeFromFilePath(codeFile, defaultTs);
      break;
    }

    if (!language || !languageNeedsLock(language)) continue;

    // Check for lock file
    const lockFile = path.join(backendDir, `${runnableId}.lock`);
    let hasLock = false;
    try {
      const s = await stat(lockFile);
      hasLock = s.size > 0;
    } catch {
      // No lock file
    }

    // Also check if the runnable YAML has inlineScript.lock
    if (!hasLock && runnable.inlineScript?.lock) {
      hasLock = await isLockResolved(runnable.inlineScript.lock, backendDir);
    }

    if (!hasLock) {
      issues.push({
        path: rawAppYamlPath,
        target: "raw_app_inline_script",
        errors: [
          `Missing lock for ${language} runnable '${runnableId}'. Run 'wmill app generate-locks' to generate locks.`,
        ],
      });
    }
  }

  // Auto-detect code files without YAML config
  for (const fileName of allFiles) {
    if (fileName.endsWith(".yaml") || fileName.endsWith(".lock")) continue;

    // Extract runnableId from code file
    let runnableId: string | null = null;
    try {
      const lang = inferContentTypeFromFilePath(fileName, defaultTs);
      if (lang) {
        // The runnableId is the filename without the extension portion
        // We need to find which extension matches
        for (const ext of exts) {
          if (fileName.endsWith(ext)) {
            runnableId = fileName.slice(0, -ext.length);
            break;
          }
        }
      }
    } catch {
      continue;
    }

    if (!runnableId || processedIds.has(runnableId)) continue;
    processedIds.add(runnableId);

    let language: string;
    try {
      language = inferContentTypeFromFilePath(fileName, defaultTs);
    } catch {
      continue;
    }

    if (!languageNeedsLock(language)) continue;

    const lockFile = path.join(backendDir, `${runnableId}.lock`);
    let hasLock = false;
    try {
      const s = await stat(lockFile);
      hasLock = s.size > 0;
    } catch {
      // No lock file
    }

    if (!hasLock) {
      issues.push({
        path: rawAppYamlPath,
        target: "raw_app_inline_script",
        errors: [
          `Missing lock for ${language} runnable '${runnableId}'. Run 'wmill app generate-locks' to generate locks.`,
        ],
      });
    }
  }

  return issues;
}

/**
 * Check for missing lock files across scripts, flow inline scripts,
 * app inline scripts, and raw app backend scripts.
 * Returns a list of issues for scripts/inline scripts that should have locks but don't.
 */
export async function checkMissingLocks(
  opts: GlobalOptions & { defaultTs?: "bun" | "deno" },
  directory?: string,
): Promise<FileIssue[]> {
  const initialCwd = process.cwd();
  const targetDirectory = directory
    ? path.resolve(initialCwd, directory)
    : process.cwd();

  const { ...syncOpts } = opts;
  const mergedOpts = await mergeConfigWithConfigFile(syncOpts);

  const ignore = await ignoreF(mergedOpts);
  const root = await FSFSElement(targetDirectory, [], false);

  const issues: FileIssue[] = [];
  const defaultTs = mergedOpts.defaultTs;
  const flowSuffix = getFolderSuffix("flow");
  const appSuffix = getFolderSuffix("app");
  const rawAppSuffix = getFolderSuffix("raw_app");

  // Collect all file paths and categorize them
  const scriptYamls: string[] = [];
  const flowYamls: { normalizedPath: string; fullPath: string }[] = [];
  const appYamls: { normalizedPath: string; fullPath: string }[] = [];
  const rawAppYamls: { normalizedPath: string; fullPath: string }[] = [];

  for await (const entry of readDirRecursiveWithIgnore(ignore, root)) {
    if (entry.isDirectory || entry.ignored) continue;

    const normalizedPath = normalizePath(entry.path);

    // Standalone script metadata files (not inside flow/app folders)
    if (
      normalizedPath.endsWith(".script.yaml") &&
      !isFlowInlineScriptPath(normalizedPath) &&
      !isAppInlineScriptPath(normalizedPath)
    ) {
      scriptYamls.push(normalizedPath);
    }

    // Flow definition files
    if (
      normalizedPath.endsWith("/flow.yaml") &&
      normalizedPath.includes(flowSuffix + "/")
    ) {
      flowYamls.push({
        normalizedPath,
        fullPath: path.join(targetDirectory, entry.path),
      });
    }

    // Normal app definition files
    if (
      normalizedPath.endsWith("/app.yaml") &&
      normalizedPath.includes(appSuffix + "/")
    ) {
      appYamls.push({
        normalizedPath,
        fullPath: path.join(targetDirectory, entry.path),
      });
    }

    // Raw app definition files
    if (
      normalizedPath.endsWith("/raw_app.yaml") &&
      normalizedPath.includes(rawAppSuffix + "/")
    ) {
      rawAppYamls.push({
        normalizedPath,
        fullPath: path.join(targetDirectory, entry.path),
      });
    }
  }

  // Check standalone scripts
  for (const yamlPath of scriptYamls) {
    const basePath = yamlPath.replace(/\.script\.yaml$/, "");

    // Find the content file to determine language
    let language: ScriptLanguage | null = null;
    for (const ext of exts) {
      try {
        await stat(path.join(targetDirectory, basePath + ext));
        language = inferContentTypeFromFilePath(basePath + ext, defaultTs);
        break;
      } catch {
        // Content file with this extension doesn't exist, try next
      }
    }

    if (language && languageNeedsLock(language)) {
      // Read the metadata to check the lock field
      try {
        const metadata = (await yamlParseFile(
          path.join(targetDirectory, yamlPath),
        )) as { lock?: string | string[] };

        const lockResolved = await isLockResolved(
          metadata?.lock,
          targetDirectory,
        );
        if (!lockResolved) {
          issues.push({
            path: yamlPath,
            target: "script",
            errors: [
              `Missing lock for ${language} script. Run 'wmill script generate-metadata' to generate locks.`,
            ],
          });
        }
      } catch (e) {
        log.debug(`Failed to parse ${yamlPath}: ${e}`);
      }
    }
  }

  // Check flow inline scripts
  for (const { normalizedPath: flowYamlPath, fullPath } of flowYamls) {
    const flowDir = path.dirname(fullPath);

    try {
      const flowFile = (await yamlParseFile(fullPath)) as {
        value?: { modules?: any[] };
      };
      if (!flowFile?.value?.modules) continue;

      const rawScripts = findRawScriptsInModules(flowFile.value.modules);

      for (const script of rawScripts) {
        if (!languageNeedsLock(script.language as ScriptLanguage)) continue;

        const lockResolved = await isLockResolved(script.lock, flowDir);
        if (!lockResolved) {
          issues.push({
            path: flowYamlPath,
            target: "flow_inline_script",
            errors: [
              `Missing lock for ${script.language} inline script '${script.id}'. Run 'wmill flow generate-locks' to generate locks.`,
            ],
          });
        }
      }
    } catch (e) {
      log.debug(`Failed to parse flow ${flowYamlPath}: ${e}`);
    }
  }

  // Check normal app inline scripts
  for (const { normalizedPath: appYamlPath, fullPath } of appYamls) {
    const appDir = path.dirname(fullPath);

    try {
      const appFile = (await yamlParseFile(fullPath)) as { value?: any };
      if (!appFile?.value) continue;

      const inlineScripts = findInlineScriptsInApp(appFile.value);
      for (const script of inlineScripts) {
        if (!languageNeedsLock(script.language)) continue;

        const lockResolved = await isLockResolved(script.lock, appDir);
        if (!lockResolved) {
          issues.push({
            path: appYamlPath,
            target: "app_inline_script",
            errors: [
              `Missing lock for ${script.language} inline script at '${script.path}'. Run 'wmill app generate-locks' to generate locks.`,
            ],
          });
        }
      }
    } catch (e) {
      log.debug(`Failed to parse app ${appYamlPath}: ${e}`);
    }
  }

  // Check raw app backend scripts
  for (const { normalizedPath: rawAppYamlPath, fullPath } of rawAppYamls) {
    const rawAppDir = path.dirname(fullPath);
    const backendDir = path.join(rawAppDir, "backend");

    try {
      await stat(backendDir);
    } catch {
      continue; // No backend folder
    }

    try {
      const runnableIssues = await checkRawAppRunnables(
        backendDir,
        rawAppYamlPath,
        defaultTs,
      );
      issues.push(...runnableIssues);
    } catch (e) {
      log.debug(`Failed to check raw app runnables ${rawAppYamlPath}: ${e}`);
    }
  }

  return issues;
}

export async function runLint(
  opts: LintOptions,
  directory?: string,
): Promise<LintReport> {
  const initialCwd = process.cwd();
  const explicitTargetDirectory = directory
    ? path.resolve(initialCwd, directory)
    : undefined;

  const { json: _json, ...syncOpts } = opts;
  const mergedOpts = await mergeConfigWithConfigFile(syncOpts);
  const targetDirectory = explicitTargetDirectory ?? process.cwd();

  const stats = await stat(targetDirectory).catch(() => null);
  if (!stats) {
    throw new Error(`Directory not found: ${targetDirectory}`);
  }
  if (!stats.isDirectory()) {
    throw new Error(`Path is not a directory: ${targetDirectory}`);
  }

  const ignore = await ignoreF(mergedOpts);
  const root = await FSFSElement(targetDirectory, [], false);
  const validator = new WindmillYamlValidator();

  const warnings: LintWarning[] = [];
  const issues: FileIssue[] = [];
  let scannedFiles = 0;
  let validatedFiles = 0;
  let validFiles = 0;
  let skippedUnsupportedFiles = 0;

  for await (const entry of readDirRecursiveWithIgnore(ignore, root)) {
    if (entry.isDirectory || entry.ignored) {
      continue;
    }
    scannedFiles += 1;

    const normalizedPath = normalizePath(entry.path);
    if (!YAML_FILE_REGEX.test(normalizedPath)) {
      continue;
    }

    if (isUnsupportedTriggerPath(normalizedPath)) {
      warnings.push({
        path: normalizedPath,
        message:
          "Unsupported trigger schema for linting (native triggers are skipped)",
      });
      skippedUnsupportedFiles += 1;
      continue;
    }

    const target = getValidationTargetFromFilename(normalizedPath);
    if (!target) {
      continue;
    }

    validatedFiles += 1;
    const content = await entry.getContentText();
    const result = validator.validate(content, target);

    const fileErrors = [
      ...formatYamlDiagnostics(result.parsed),
      ...result.errors.map((error) => formatValidationError(error)),
    ];
    if (fileErrors.length > 0) {
      issues.push({
        path: normalizedPath,
        target: formatTarget(target),
        errors: fileErrors,
      });
    } else {
      validFiles += 1;
    }
  }

  // Check for missing locks if --locks-required is set
  if (opts.locksRequired) {
    const lockIssues = await checkMissingLocks(opts, explicitTargetDirectory);
    issues.push(...lockIssues);
  }

  const invalidFiles = issues.length;
  const shouldFail = invalidFiles > 0 ||
    (!!opts.failOnWarn && warnings.length > 0);

  return {
    scannedFiles,
    validatedFiles,
    validFiles,
    invalidFiles,
    skippedUnsupportedFiles,
    warnings,
    issues,
    success: !shouldFail,
    exitCode: shouldFail ? 1 : 0,
  };
}

export function printReport(report: LintReport, jsonOutput: boolean) {
  if (jsonOutput) {
    console.log(JSON.stringify(report, null, 2));
    return;
  }

  if (report.warnings.length > 0) {
    log.info(colors.yellow(`\n⚠️  Warnings (${report.warnings.length}):`));
    for (const warning of report.warnings) {
      log.info(colors.yellow(`  - ${warning.path}: ${warning.message}`));
    }
  }

  if (report.issues.length > 0) {
    log.info(colors.red(`\n❌ Invalid files (${report.issues.length}):`));
    for (const issue of report.issues) {
      log.info(colors.red(`  - ${issue.path} (${issue.target})`));
      for (const error of issue.errors) {
        log.info(colors.red(`    • ${error}`));
      }
    }
  }

  if (report.success) {
    log.info(
      colors.green(
        `\n✅ Lint passed (${report.validatedFiles} file(s) validated, ${report.validFiles} valid)\n`,
      ),
    );
  } else {
    log.info(
      colors.red(
        `\n❌ Lint failed (${report.invalidFiles} invalid file(s), ${report.warnings.length} warning(s))\n`,
      ),
    );
  }
}

async function lint(opts: LintOptions, directory?: string) {
  try {
    const report = await runLint(opts, directory);
    printReport(report, !!opts.json);
    if (report.exitCode !== 0) {
      process.exit(report.exitCode);
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    if (opts.json) {
      console.log(
        JSON.stringify(
          {
            success: false,
            exitCode: 1,
            error: message,
          },
          null,
          2,
        ),
      );
    } else {
      log.error(colors.red(`❌ ${message}`));
    }
    process.exit(1);
  }
}

const command = new Command()
  .description(
    "Validate Windmill flow, schedule, and trigger YAML files in a directory",
  )
  .arguments("[directory:string]")
  .option("--json", "Output results in JSON format")
  .option("--fail-on-warn", "Exit with code 1 when warnings are emitted")
  .option(
    "--locks-required",
    "Fail if scripts or flow inline scripts that need locks have no locks",
  )
  .action(lint as any);

export default command;
