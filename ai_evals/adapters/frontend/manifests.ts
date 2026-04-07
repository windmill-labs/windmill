import { readdir, readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  loadEvalCaseSummaries,
  type EvalSurfaceName
} from "../shared/evalCases";

export type FrontendSurfaceName = "frontend-flow" | "frontend-app" | "frontend-script";

interface FrontendCaseManifest {
  id: string;
  title: string;
}

interface FrontendVariantManifest {
  id: string;
  description?: string;
}

const REPO_ROOT = fileURLToPath(new URL("../../../", import.meta.url));

export async function loadFrontendCases(
  surface: FrontendSurfaceName
): Promise<FrontendCaseManifest[]> {
  return loadEvalCaseSummaries(surface as EvalSurfaceName).map((entry) => ({
    id: entry.id,
    title: entry.title
  }));
}

export async function loadFrontendVariants(
  surface: FrontendSurfaceName
): Promise<FrontendVariantManifest[]> {
  const variantsDir = path.join(
    REPO_ROOT,
    "ai_evals",
    "variants",
    "frontend",
    surfaceToManifestName(surface)
  );
  const filenames = (await readdir(variantsDir))
    .filter((entry) => entry.endsWith(".json"))
    .sort((left, right) => left.localeCompare(right));

  return await Promise.all(
    filenames.map(async (filename) => {
      const raw = await readFile(path.join(variantsDir, filename), "utf8");
      return JSON.parse(raw) as FrontendVariantManifest;
    })
  );
}

function surfaceToManifestName(surface: FrontendSurfaceName): "flow" | "app" | "script" {
  if (surface === "frontend-flow") {
    return "flow";
  }
  if (surface === "frontend-app") {
    return "app";
  }
  return "script";
}
