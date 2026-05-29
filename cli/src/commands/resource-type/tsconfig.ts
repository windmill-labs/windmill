import { execSync } from "node:child_process";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import process from "node:process";

import { colors } from "@cliffy/ansi/colors";
import * as log from "../../core/log.ts";
import { readConfigFile } from "../../core/conf.ts";

/**
 * Local-friendly aliases for the absolute workspace import paths `/f/...` and
 * `/u/...`. Unlike the `/`-prefixed form (which every local tool treats as a
 * filesystem-root path), `$f/`/`$u/` are bare specifiers that can be remapped to
 * the on-disk `f/`/`u/` folders via tsconfig `paths` and Deno import maps, so the
 * same import resolves both on the Windmill worker and in a local editor.
 */
const WORKSPACE_IMPORT_ALIASES: Record<string, string> = {
  $f: "f",
  $u: "u",
};

export async function generateTsconfigForIde() {
  await generateDenoConfigForIde();

  // Map "$f/*" -> ["./f/*"], "$u/*" -> ["./u/*"] so the editor resolves workspace
  // imports against the local script folders.
  const paths: Record<string, string[]> = {};
  for (const [alias, dir] of Object.entries(WORKSPACE_IMPORT_ALIASES)) {
    paths[`${alias}/*`] = [`./${dir}/*`];
  }

  const tsconfigPath = path.join(process.cwd(), "tsconfig.json");
  if (existsSync(tsconfigPath)) {
    mergeWorkspaceAliasesIntoTsconfig(tsconfigPath, paths);
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
      // Workspace imports carry an explicit `.ts` extension (e.g. "$f/foo/bar.ts");
      // allow it so the editor doesn't flag every cross-script import.
      allowImportingTsExtensions: true,
      noEmit: true,
      strict: false,
      // No `baseUrl`: with moduleResolution "bundler" the `paths` patterns resolve
      // relative to this file, and `baseUrl` is deprecated in TypeScript 7+.
      paths,
    },
    include: ["**/*.ts", "rt.d.ts"],
  };

  if (bunTypesAvailable) {
    tsconfig.compilerOptions.types = ["bun-types"];
  }

  writeFileSync(tsconfigPath, JSON.stringify(tsconfig, null, 2) + "\n");
  log.info(colors.green("Created tsconfig.json for IDE type support."));
}

/**
 * Add the `$f/`/`$u/` path aliases to an existing tsconfig.json so workspace
 * imports resolve in the editor. Only fills in missing keys — never overwrites
 * the user's existing compilerOptions. Falls back to printing the snippet when
 * the file can't be parsed as JSON (e.g. it contains comments).
 */
function mergeWorkspaceAliasesIntoTsconfig(
  tsconfigPath: string,
  paths: Record<string, string[]>
) {
  let tsconfig: {
    compilerOptions?: Record<string, unknown>;
    [key: string]: unknown;
  };
  try {
    tsconfig = JSON.parse(readFileSync(tsconfigPath, "utf-8"));
  } catch {
    log.info(
      colors.gray(
        "tsconfig.json already exists but couldn't be parsed. Add these for $f//$u/ resolution:\n" +
          JSON.stringify(
            {
              compilerOptions: {
                allowImportingTsExtensions: true,
                paths,
              },
            },
            null,
            2
          )
      )
    );
    return;
  }

  const compilerOptions = { ...(tsconfig.compilerOptions ?? {}) };
  const existingPaths = {
    ...((compilerOptions.paths as Record<string, string[]>) ?? {}),
  };
  let changed = false;

  for (const [key, value] of Object.entries(paths)) {
    if (existingPaths[key] === undefined) {
      existingPaths[key] = value;
      changed = true;
    }
  }
  // `allowImportingTsExtensions` is only valid alongside noEmit/emitDeclarationOnly;
  // don't introduce a compiler error into a user's emitting config.
  if (
    compilerOptions.allowImportingTsExtensions === undefined &&
    (compilerOptions.noEmit || compilerOptions.emitDeclarationOnly)
  ) {
    compilerOptions.allowImportingTsExtensions = true;
    changed = true;
  }

  if (!changed) {
    log.info(colors.gray("tsconfig.json already has $f//$u/ aliases, skipping"));
    return;
  }

  compilerOptions.paths = existingPaths;
  tsconfig.compilerOptions = compilerOptions;
  writeFileSync(tsconfigPath, JSON.stringify(tsconfig, null, 2) + "\n");
  log.info(colors.green("Added $f//$u/ path aliases to tsconfig.json."));
}

/**
 * Generate a `deno.json` import map so the Deno LSP (which ignores tsconfig.json)
 * resolves `$f/`/`$u/` workspace imports against the local script folders. Only
 * runs for Deno-default projects. Merges into an existing `deno.json` when safe;
 * otherwise prints the snippet for the user to add manually.
 */
async function generateDenoConfigForIde() {
  let defaultTs: "bun" | "deno" = "bun";
  try {
    const conf = await readConfigFile({ warnIfMissing: false });
    if (conf?.defaultTs === "deno") {
      defaultTs = "deno";
    }
  } catch {
    // fall back to bun if wmill.yaml is missing or unreadable
  }
  if (defaultTs !== "deno") {
    return;
  }

  // Import-map prefix keys must end with "/": "$f/" -> "./f/", "$u/" -> "./u/".
  const aliasImports: Record<string, string> = {};
  for (const [alias, dir] of Object.entries(WORKSPACE_IMPORT_ALIASES)) {
    aliasImports[`${alias}/`] = `./${dir}/`;
  }

  const denoJsonPath = path.join(process.cwd(), "deno.json");
  const denoJsoncPath = path.join(process.cwd(), "deno.jsonc");

  // Don't risk mangling a jsonc file with comments — just show the snippet.
  if (existsSync(denoJsoncPath)) {
    log.info(
      colors.gray(
        "deno.jsonc already exists. Add these imports for $f//$u/ resolution:\n" +
          JSON.stringify({ imports: aliasImports }, null, 2)
      )
    );
    return;
  }

  if (existsSync(denoJsonPath)) {
    try {
      const existing = JSON.parse(readFileSync(denoJsonPath, "utf-8"));
      const imports = { ...(existing.imports ?? {}) };
      let changed = false;
      for (const [key, value] of Object.entries(aliasImports)) {
        if (imports[key] === undefined) {
          imports[key] = value;
          changed = true;
        }
      }
      if (!changed) {
        log.info(colors.gray("deno.json already has $f//$u/ imports, skipping"));
        return;
      }
      existing.imports = imports;
      writeFileSync(denoJsonPath, JSON.stringify(existing, null, 2) + "\n");
      log.info(colors.green("Added $f//$u/ imports to deno.json."));
    } catch {
      log.warn(
        "Could not parse deno.json. Add these imports manually for $f//$u/ resolution:\n" +
          JSON.stringify({ imports: aliasImports }, null, 2)
      );
    }
    return;
  }

  writeFileSync(
    denoJsonPath,
    JSON.stringify({ imports: aliasImports }, null, 2) + "\n"
  );
  log.info(colors.green("Created deno.json with $f//$u/ imports for IDE type support."));
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
