// deno-lint-ignore-file no-explicit-any
import * as fs from "node:fs";
import * as path from "node:path";
import process from "node:process";
import { Command, colors, log, yamlParseFile } from "../../../deps.ts";
import { GlobalOptions } from "../../types.ts";
import { createBundle } from "./bundle.ts";

interface LintOptions extends GlobalOptions {
  fix?: boolean;
}

interface LintResult {
  valid: boolean;
  errors: string[];
  warnings: string[];
}

/**
 * Validates the structure of raw_app.yaml
 */
function validateRawAppYaml(appData: any): {
  errors: string[];
  warnings: string[];
} {
  const errors: string[] = [];
  const warnings: string[] = [];

  // Check required fields
  if (!appData.summary) {
    errors.push("Missing required field: 'summary'");
  } else if (typeof appData.summary !== "string") {
    errors.push("Field 'summary' must be a string");
  }

  if (!appData.runnables) {
    errors.push("Missing required field: 'runnables'");
  } else if (
    typeof appData.runnables !== "object" ||
    Array.isArray(appData.runnables)
  ) {
    errors.push("Field 'runnables' must be an object");
  }

  // Check optional but recommended fields
  // if (!appData.custom_path) {
  //   warnings.push("No 'custom_path' specified - app path may not be what you expect");
  // }

  return { errors, warnings };
}

/**
 * Checks if the app can be built successfully
 */
async function validateBuild(
  appDir: string
): Promise<{ errors: string[]; warnings: string[] }> {
  const errors: string[] = [];
  const warnings: string[] = [];

  try {
    log.info(colors.blue("🔨 Testing build..."));

    // Try to create a bundle - this will validate that all dependencies are in place
    await createBundle({
      production: true,
      minify: false,
    });

    log.info(colors.green("✅ Build successful"));
  } catch (error: any) {
    errors.push(`Build failed: ${error.message}`);
  }

  return { errors, warnings };
}

/**
 * Validates a raw app folder
 */
async function lintRawApp(
  appDir: string,
  opts: LintOptions
): Promise<LintResult> {
  const errors: string[] = [];
  const warnings: string[] = [];

  // Check if we're in a .raw_app folder
  const currentDirName = path.basename(appDir);
  if (!currentDirName.endsWith(".raw_app")) {
    errors.push(
      `Not a raw app folder: '${currentDirName}' does not end with '.raw_app'`
    );
    return { valid: false, errors, warnings };
  }

  // Check if raw_app.yaml exists
  const rawAppPath = path.join(appDir, "raw_app.yaml");
  if (!fs.existsSync(rawAppPath)) {
    errors.push("Missing raw_app.yaml file");
    return { valid: false, errors, warnings };
  }

  log.info(colors.blue("📋 Validating raw_app.yaml structure..."));

  // Parse and validate raw_app.yaml
  let appData: any;
  try {
    appData = await yamlParseFile(rawAppPath);
  } catch (error: any) {
    errors.push(`Failed to parse raw_app.yaml: ${error.message}`);
    return { valid: false, errors, warnings };
  }

  const yamlValidation = validateRawAppYaml(appData);
  errors.push(...yamlValidation.errors);
  warnings.push(...yamlValidation.warnings);

  if (errors.length > 0) {
    return { valid: false, errors, warnings };
  }

  log.info(colors.green("✅ raw_app.yaml structure is valid"));

  // Validate build
  const buildValidation = await validateBuild(appDir);
  errors.push(...buildValidation.errors);
  warnings.push(...buildValidation.warnings);

  return {
    valid: errors.length === 0,
    errors,
    warnings,
  };
}

/**
 * Main lint command
 */
async function lint(opts: LintOptions, appFolder?: string) {
  const targetDir = appFolder ?? process.cwd();

  log.info(colors.bold.blue(`\n🔍 Linting raw app: ${targetDir}\n`));

  const result = await lintRawApp(targetDir, opts);

  // Display results
  if (result.warnings.length > 0) {
    log.info(colors.yellow("\n⚠️  Warnings:"));
    result.warnings.forEach((warning) => {
      log.info(colors.yellow(`  - ${warning}`));
    });
  }

  if (result.errors.length > 0) {
    log.info(colors.red("\n❌ Errors:"));
    result.errors.forEach((error) => {
      log.info(colors.red(`  - ${error}`));
    });
    log.info(colors.red("\n❌ Lint failed\n"));
    Deno.exit(1);
  }

  log.info(colors.green("\n✅ All checks passed\n"));
}

const command = new Command()
  .description("Lint a raw app folder to validate structure and buildability")
  .arguments("[app_folder:string]")
  .option("--fix", "Attempt to fix common issues (not implemented yet)")
  .action(lint as any);

export default command;
