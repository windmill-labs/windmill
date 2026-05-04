import { execSync } from "node:child_process";
import { existsSync, writeFileSync } from "node:fs";
import path from "node:path";
import process from "node:process";

import { colors } from "@cliffy/ansi/colors";
import * as log from "../../core/log.ts";
import { readConfigFile } from "../../core/conf.ts";

export async function generateTsconfigForIde() {
  const tsconfigPath = path.join(process.cwd(), "tsconfig.json");
  if (existsSync(tsconfigPath)) {
    log.info(colors.gray("tsconfig.json already exists, skipping"));
    return;
  }

  let defaultTs: "bun" | "deno" = "bun";
  try {
    const conf = await readConfigFile({ warnIfMissing: false });
    if (conf?.defaultTs === "deno") {
      defaultTs = "deno";
    }
  } catch {
    // fall back to bun if wmill.yaml is missing or unreadable
  }

  // Only reference bun-types in tsconfig if it's actually available; otherwise
  // the IDE will flag the missing type definitions.
  const bunTypesAvailable =
    defaultTs === "bun" ? ensureBunTypesAvailable() : false;

  const tsconfig: {
    compilerOptions: Record<string, unknown>;
    include: string[];
  } = {
    compilerOptions: {
      target: "ESNext",
      module: "ESNext",
      moduleResolution: "bundler",
      noEmit: true,
      strict: false,
    },
    include: ["**/*.ts", "rt.d.ts"],
  };

  if (bunTypesAvailable) {
    tsconfig.compilerOptions.types = ["bun-types"];
  }

  writeFileSync(tsconfigPath, JSON.stringify(tsconfig, null, 2) + "\n");
  log.info(colors.green("Created tsconfig.json for IDE type support."));
}

function ensureBunTypesAvailable(): boolean {
  const cwd = process.cwd();
  if (existsSync(path.join(cwd, "node_modules", "bun-types"))) {
    return true;
  }

  try {
    execSync("bun --version", { stdio: "ignore" });
  } catch {
    log.info(
      "Install bun (https://bun.sh) then run 'bun add -d bun-types' and add \"types\": [\"bun-types\"] to tsconfig.json for Bun API autocompletion."
    );
    return false;
  }

  try {
    log.info(
      colors.yellow("Installing bun-types with 'bun add -d bun-types'...")
    );
    execSync("bun add -d bun-types", { stdio: "inherit" });
    log.info(colors.green("Installed bun-types."));
    return true;
  } catch (e) {
    log.warn(
      `Failed to install bun-types automatically: ${
        e instanceof Error ? e.message : e
      }`
    );
    log.info(
      "Run 'bun add -d bun-types' manually and add \"types\": [\"bun-types\"] to tsconfig.json for Bun API autocompletion."
    );
    return false;
  }
}
