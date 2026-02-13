import { colors, Command, log, path, SEP } from "../../../deps.ts";
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
} from "npm:windmill-yaml-validator@1.1.0";

interface LintOptions extends GlobalOptions {
  json?: boolean;
  failOnWarn?: boolean;
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

function formatYamlDiagnostics(parsed: unknown): string[] {
  const diagnostics = (parsed as { diagnostics?: Array<{ message?: string }> })
    ?.diagnostics;
  if (!Array.isArray(diagnostics) || diagnostics.length === 0) {
    return [];
  }
  return diagnostics.map((d) => d?.message || "Invalid YAML document");
}

export async function runLint(
  opts: LintOptions,
  directory?: string,
): Promise<LintReport> {
  const initialCwd = Deno.cwd();
  const explicitTargetDirectory = directory
    ? path.resolve(initialCwd, directory)
    : undefined;

  const mergedOpts = await mergeConfigWithConfigFile({
    ...opts,
    json: false,
  });
  const targetDirectory = explicitTargetDirectory ?? Deno.cwd();

  const stats = await Deno.stat(targetDirectory).catch(() => null);
  if (!stats) {
    throw new Error(`Directory not found: ${targetDirectory}`);
  }
  if (!stats.isDirectory) {
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

function printReport(report: LintReport, jsonOutput: boolean) {
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
      Deno.exit(report.exitCode);
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
    Deno.exit(1);
  }
}

const command = new Command()
  .description(
    "Validate Windmill flow, schedule, and trigger YAML files in a directory",
  )
  .arguments("[directory:string]")
  .option("--json", "Output results in JSON format")
  .option("--fail-on-warn", "Exit with code 1 when warnings are emitted")
  .action(lint as any);

export default command;
