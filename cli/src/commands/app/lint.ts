import * as fs from "node:fs";
import * as path from "node:path";
import process from "node:process";
import { Command } from "@cliffy/command";
import { colors } from "@cliffy/ansi/colors";
import * as log from "../../core/log.ts";
import { yamlParseFile } from "../../utils/yaml.ts";
import { GlobalOptions } from "../../types.ts";
import { createBundle } from "./bundle.ts";
import { APP_BACKEND_FOLDER } from "./app_metadata.ts";
import { loadRunnablesFromBackend } from "./raw_apps.ts";
import {
  getFolderSuffix,
  hasFolderSuffix,
  loadNonDottedPathsSetting,
} from "../../utils/resource_folders.ts";

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

  // Note: 'runnables' is no longer required in raw_app.yaml
  // Runnables can be stored in separate files in the backend folder

  return { errors, warnings };
}

/**
 * Validates that runnables exist either in backend/*.yaml files or in raw_app.yaml
 */
async function validateRunnables(
  appDir: string,
  appData: any,
): Promise<{ errors: string[]; warnings: string[] }> {
  const errors: string[] = [];
  const warnings: string[] = [];

  const backendPath = path.join(appDir, APP_BACKEND_FOLDER);

  // Load runnables from separate files (new format)
  const runnablesFromBackend = await loadRunnablesFromBackend(backendPath);
  const hasBackendRunnables = Object.keys(runnablesFromBackend).length > 0;

  // Check for runnables in raw_app.yaml (old format)
  const hasYamlRunnables = appData.runnables &&
    typeof appData.runnables === "object" &&
    !Array.isArray(appData.runnables) &&
    Object.keys(appData.runnables).length > 0;

  if (!hasBackendRunnables && !hasYamlRunnables) {
    errors.push(
      "No runnables found. Expected either:\n" +
        "  - Runnable YAML files in the 'backend/' folder (e.g., backend/myRunnable.yaml)\n" +
        "  - Or a 'runnables' field in raw_app.yaml (legacy format)",
    );
  } else if (hasBackendRunnables) {
    log.info(
      colors.gray(
        `  Found ${
          Object.keys(runnablesFromBackend).length
        } runnable(s) in backend folder`,
      ),
    );
  } else if (hasYamlRunnables) {
    log.info(
      colors.gray(
        `  Found ${
          Object.keys(appData.runnables).length
        } runnable(s) in raw_app.yaml (legacy format)`,
      ),
    );
    warnings.push(
      "Using legacy format with runnables in raw_app.yaml. Consider migrating to separate files in backend/",
    );
  }

  return { errors, warnings };
}

/**
 * Checks if the app can be built successfully
 */
async function validateBuild(
  appDir: string,
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
  opts: LintOptions,
): Promise<LintResult> {
  const errors: string[] = [];
  const warnings: string[] = [];

  // Check if we're in a .raw_app folder
  const currentDirName = path.basename(appDir);
  if (!hasFolderSuffix(currentDirName, "raw_app")) {
    errors.push(
      `Not a raw app folder: '${currentDirName}' does not end with '${
        getFolderSuffix("raw_app")
      }'`,
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

  // Validate runnables (either in backend folder or in raw_app.yaml)
  log.info(colors.blue("📋 Validating runnables..."));
  const runnablesValidation = await validateRunnables(appDir, appData);
  errors.push(...runnablesValidation.errors);
  warnings.push(...runnablesValidation.warnings);

  if (errors.length > 0) {
    return { valid: false, errors, warnings };
  }

  log.info(colors.green("✅ Runnables are valid"));

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
  // Load nonDottedPaths setting before using folder suffix functions
  await loadNonDottedPathsSetting();

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
    process.exit(1);
  }

  log.info(colors.green("\n✅ All checks passed\n"));
}

const command = new Command()
  .description("Lint a raw app folder to validate structure and buildability")
  .arguments("[app_folder:string]")
  .option("--fix", "Attempt to fix common issues (not implemented yet)")
  .action(lint as any);

export default command;
