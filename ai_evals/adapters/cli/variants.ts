import { mkdir, readFile, readdir, rm, writeFile } from "fs/promises";
import { join, resolve } from "path";
import { fileURLToPath } from "url";
import { getGeneratedSkillsSource } from "./runtime";
import {
  WMILL_INIT_AI_AGENTS_SOURCE_ENV,
  WMILL_INIT_AI_CLAUDE_SOURCE_ENV,
  WMILL_INIT_AI_SKILLS_SOURCE_ENV,
  writeAiGuidanceFiles
} from "../../../cli/src/guidance/writer.ts";

type CliVariantSource =
  | {
      type: "generated";
    }
  | {
      type: "path";
      path: string;
    };

interface CliVariantManifest {
  id: string;
  description?: string;
  skillsSource: CliVariantSource;
  agentsSourcePath?: string;
  claudeSourcePath?: string;
}

export interface CliVariant {
  id: string;
  description?: string;
  skillsSourcePath: string;
  agentsSourcePath?: string;
  claudeSourcePath?: string;
}

export interface CliVariantSnapshotResult {
  variantId: string;
  manifestPath: string;
  snapshotDir: string;
  description: string;
  usedOverrides: {
    skillsSourcePath?: string;
    agentsSourcePath?: string;
    claudeSourcePath?: string;
  };
}

const VARIANTS_DIR = fileURLToPath(new URL("../../variants/cli", import.meta.url));
const SNAPSHOTS_DIR = join(VARIANTS_DIR, "snapshots");

export async function loadCliVariants(): Promise<CliVariant[]> {
  const filenames = (await readdir(VARIANTS_DIR))
    .filter((entry) => entry.endsWith(".json"))
    .sort((left, right) => left.localeCompare(right));

  const variants: CliVariant[] = [];

  for (const filename of filenames) {
    const manifestPath = join(VARIANTS_DIR, filename);
    const raw = await readFile(manifestPath, "utf8");
    const parsed = JSON.parse(raw) as CliVariantManifest;

    if (!parsed.id) {
      throw new Error(`Missing variant id in ${manifestPath}`);
    }

    variants.push({
      id: parsed.id,
      description: parsed.description,
      skillsSourcePath: resolveVariantSkillsSource(parsed.skillsSource, manifestPath),
      agentsSourcePath: resolveOptionalManifestPath(parsed.agentsSourcePath, manifestPath),
      claudeSourcePath: resolveOptionalManifestPath(parsed.claudeSourcePath, manifestPath)
    });
  }

  return variants;
}

export async function loadCliVariantById(variantId: string): Promise<CliVariant> {
  const variants = await loadCliVariants();
  const variant = variants.find((entry) => entry.id === variantId);
  if (!variant) {
    throw new Error(`Unknown CLI variant: ${variantId}`);
  }
  return variant;
}

export async function snapshotCliVariant(options: {
  variantId: string;
  description?: string;
}): Promise<CliVariantSnapshotResult> {
  validateVariantId(options.variantId);

  const snapshotDir = join(SNAPSHOTS_DIR, options.variantId);
  const manifestPath = join(VARIANTS_DIR, `${options.variantId}.json`);
  const usedOverrides = {
    skillsSourcePath: process.env[WMILL_INIT_AI_SKILLS_SOURCE_ENV],
    agentsSourcePath: process.env[WMILL_INIT_AI_AGENTS_SOURCE_ENV],
    claudeSourcePath: process.env[WMILL_INIT_AI_CLAUDE_SOURCE_ENV]
  };

  await rm(snapshotDir, { recursive: true, force: true });
  await mkdir(snapshotDir, { recursive: true });
  await writeAiGuidanceFiles({
    targetDir: snapshotDir,
    nonDottedPaths: true,
    overwriteProjectGuidance: true,
    skillsSourcePath: usedOverrides.skillsSourcePath,
    agentsSourcePath: usedOverrides.agentsSourcePath,
    claudeSourcePath: usedOverrides.claudeSourcePath
  });

  const manifest: CliVariantManifest = {
    id: options.variantId,
    description:
      options.description ??
      `Snapshot of the current CLI guidance bundle stored under snapshots/${options.variantId}.`,
    skillsSource: {
      type: "path",
      path: `./snapshots/${options.variantId}/.claude/skills`
    },
    agentsSourcePath: `./snapshots/${options.variantId}/AGENTS.md`,
    claudeSourcePath: `./snapshots/${options.variantId}/CLAUDE.md`
  };

  await writeFile(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, "utf8");

  return {
    variantId: options.variantId,
    manifestPath,
    snapshotDir,
    description: manifest.description ?? "",
    usedOverrides
  };
}

function resolveVariantSkillsSource(
  skillsSource: CliVariantSource | undefined,
  manifestPath: string
): string {
  if (!skillsSource || skillsSource.type === "generated") {
    return getGeneratedSkillsSource();
  }

  return resolve(join(manifestPath, ".."), skillsSource.path);
}

function resolveOptionalManifestPath(
  inputPath: string | undefined,
  manifestPath: string
): string | undefined {
  if (!inputPath) {
    return undefined;
  }

  return resolve(join(manifestPath, ".."), inputPath);
}

function validateVariantId(variantId: string): void {
  if (!/^[a-z0-9][a-z0-9_-]*$/i.test(variantId)) {
    throw new Error(
      `Invalid variant id '${variantId}'. Use only letters, numbers, '-' and '_'`
    );
  }
}
