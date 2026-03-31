import { readdir, readFile } from "fs/promises";
import { join, resolve } from "path";
import { fileURLToPath } from "url";
import { getGeneratedSkillsSource } from "./runtime";

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

const VARIANTS_DIR = fileURLToPath(new URL("../../variants/cli", import.meta.url));

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
