import { Command } from "@cliffy/command";
import * as log from "../../core/log.ts";
import { colors } from "@cliffy/ansi/colors";
import { readFile, writeFile } from "node:fs/promises";
import { stringify as yamlStringify } from "yaml";
import {
  formatConfigReference,
  formatConfigReferenceJson,
} from "../init/template.ts";
import {
  getWmillYamlPath,
  convertGitBranchesToWorkspaces,
} from "../../core/conf.ts";
import { yamlParseFile } from "../../utils/yaml.ts";

interface ConfigOptions {
  json?: boolean;
}

async function configAction(opts: ConfigOptions) {
  if (opts.json) {
    console.log(formatConfigReferenceJson());
  } else {
    log.info(formatConfigReference());
  }
}

async function migrateAction() {
  const wmillYamlPath = getWmillYamlPath();
  if (!wmillYamlPath) {
    log.error("No wmill.yaml found. Nothing to migrate.");
    return;
  }

  const conf = (await yamlParseFile(wmillYamlPath)) as Record<string, any>;
  if (!conf) {
    log.error("wmill.yaml is empty. Nothing to migrate.");
    return;
  }

  // Collect all legacy keys present
  const legacyKeys: string[] = [];
  for (const key of ["gitBranches", "environments", "git_branches"]) {
    if (key in conf && conf[key]) {
      legacyKeys.push(key);
    }
  }

  // If no legacy keys, check if already migrated
  if (legacyKeys.length === 0) {
    if ("workspaces" in conf) {
      log.info("Already using 'workspaces' format. No migration needed.");
    } else {
      log.info(
        "No gitBranches/environments/git_branches found. Nothing to migrate."
      );
    }
    return;
  }

  // Use the first legacy key found (priority order)
  const legacyKey = legacyKeys[0];
  const legacyData = conf[legacyKey];

  // Convert
  const workspaces = convertGitBranchesToWorkspaces(legacyData);
  const wsNames = Object.keys(workspaces).filter(
    (k) => k !== "commonSpecificItems"
  );

  // Build new config: remove ALL legacy keys, add/merge workspaces
  const newConf = { ...conf };
  for (const key of legacyKeys) {
    delete newConf[key];
  }
  if (newConf.workspaces) {
    // Merge: legacy entries into existing workspaces (don't overwrite existing)
    for (const [name, entry] of Object.entries(workspaces)) {
      if (!(newConf.workspaces as any)[name]) {
        (newConf.workspaces as any)[name] = entry;
      }
    }
  } else {
    newConf.workspaces = workspaces;
  }

  await writeFile(wmillYamlPath, yamlStringify(newConf), "utf-8");
  log.info(
    colors.green(
      `✅ Migrated '${legacyKey}' to 'workspaces' in ${wmillYamlPath}`
    )
  );
  if (wsNames.length > 0) {
    log.info(`   Workspace entries: ${wsNames.join(", ")}`);
  }
  if (legacyKeys.length > 1) {
    log.info(
      `   Also removed additional legacy keys: ${legacyKeys.slice(1).join(", ")}`
    );
  }
}

const command = new Command()
  .name("config")
  .description("Show all available wmill.yaml configuration options")
  .option("--json", "Output as JSON for programmatic consumption")
  .action(configAction as any)
  .command("migrate")
  .description(
    "Migrate wmill.yaml from gitBranches/environments to workspaces format"
  )
  .action(migrateAction as any);

export default command;
