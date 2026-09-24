import { createHash } from "node:crypto";
import * as path from "node:path";
import { readFile } from "node:fs/promises";
import type { BuildOptions } from "esbuild";
import { Codebase, SyncOptions } from "../core/conf.ts";
import * as log from "../core/log.ts";
import { getEsbuild } from "./esbuild_loader.ts";
import { digestDir, generateHash, generateHashFromBuffer } from "./utils.ts";

export type SyncCodebase = Codebase & {
  getDigest: (scriptPath: string, forceTar?: boolean) => Promise<string>;
  /** Computes the digests of many scripts in one esbuild pass, ahead of `getDigest`. */
  primeDigests: (scriptPaths: string[]) => Promise<void>;
};

export function codebaseBuildOptions(codebase: Codebase): BuildOptions {
  const format = codebase.format ?? "cjs";
  return {
    format: format,
    bundle: true,
    write: false,
    external: codebase.external,
    inject: codebase.inject,
    define: codebase.define,
    loader: codebase.loader ?? { ".node": "file" },
    outdir: "/",
    platform: "node",
    packages: "bundle",
    target: format == "cjs" ? "node20.15.1" : "esnext",
    banner: codebase.banner,
  };
}

// Hashes each script's bundle rather than the files it imports: the bundle is
// what gets deployed, so edits that tree-shaking drops (an unused barrel export)
// leave the digest alone, while inlined values (TS enums) still change it.
// Other files a build emits (e.g. `.node` loaded as "file") are referenced from
// the bundle by content hash, so the bundle alone covers them.
async function computeBundleDigests(
  codebase: Codebase,
  scriptPaths: string[],
  assetsHash: () => Promise<string>
): Promise<Map<string, string>> {
  const esbuild = await getEsbuild();
  const cwd = process.cwd();
  const result = await esbuild.build({
    ...codebaseBuildOptions(codebase),
    entryPoints: scriptPaths,
    absWorkingDir: cwd,
    // esbuild names identifiers after the output path, which otherwise depends
    // on the common ancestor of all entry points: a script built alone and in a
    // batch would get different bundles, hence different digests.
    outbase: cwd,
    metafile: true,
    logLevel: "silent",
  });

  const outputFiles = new Map(result.outputFiles.map((f) => [f.path, f]));
  const suffix = (await assetsHash()) + JSON.stringify(codebase);
  const digests = new Map<string, string>();
  for (const [outputPath, output] of Object.entries(result.metafile!.outputs)) {
    if (!output.entryPoint) continue;
    const file = outputFiles.get(path.resolve(cwd, outputPath));
    if (!file) continue;
    const bundleHash = createHash("sha256").update(file.contents).digest("hex");
    digests.set(
      path.resolve(cwd, output.entryPoint),
      await generateHash(bundleHash + suffix)
    );
  }
  return digests;
}

export function listSyncCodebases(options: SyncOptions): SyncCodebase[] {
  const res: SyncCodebase[] = [];
  const nb_codebase = options?.codebases?.length ?? 0;
  if (nb_codebase > 0) {
    log.info(
      `Found ${nb_codebase} codebases: ${options?.codebases
        ?.map((c) => c.relative_path)
        .join(", ")}`
    );
  }
  for (const codebase of options?.codebases ?? []) {
    const hasAssets =
      Array.isArray(codebase.assets) && codebase.assets.length > 0;
    const digests = new Map<string, Promise<string>>();

    let assetsDigest: Promise<string> | undefined = undefined;
    const assetsHash = () => {
      if (!assetsDigest) {
        assetsDigest = Promise.all(
          (codebase.assets ?? []).map(
            async (asset) =>
              `${asset.from}:${asset.to}:${await generateHashFromBuffer(
                await readFile(asset.from)
              )}`
          )
        ).then((hashes) => hashes.join("\n"));
      }
      return assetsDigest;
    };

    // A custom bundler's inputs are unknown, so its scripts share one digest
    // of the whole `relative_path`.
    let dirDigest: Promise<string> | undefined = undefined;
    const getDirDigest = () => {
      if (!dirDigest) {
        dirDigest = (async () => {
          const d = await digestDir(
            codebase.relative_path,
            JSON.stringify(codebase)
          );
          log.info(`Codebase ${codebase.relative_path}, digest: ${d}`);
          return d;
        })();
      }
      return dirDigest;
    };

    const digestAlone = async (scriptPath: string): Promise<string> => {
      try {
        const single = await computeBundleDigests(
          codebase,
          [scriptPath],
          assetsHash
        );
        const digest = single.get(path.resolve(scriptPath));
        if (digest) return digest;
        throw new Error("esbuild reported no output for it");
      } catch (e) {
        // Falling back keeps a script that no longer bundles visible as changed,
        // so the push surfaces the bundling error instead of the diff.
        log.warn(
          `Could not compute the bundle digest of ${scriptPath}, falling back to the digest of ${codebase.relative_path}: ${e}`
        );
        return await getDirDigest();
      }
    };

    const primeDigests = async (scriptPaths: string[]) => {
      if (codebase.customBundler) return;
      const pending = [
        ...new Set(scriptPaths.map((p) => path.resolve(p))),
      ].filter((p) => !digests.has(p));
      if (pending.length == 0) return;

      const startTime = performance.now();
      const batch = computeBundleDigests(codebase, pending, assetsHash);
      for (const p of pending) {
        digests.set(
          p,
          batch.then(
            (all) => all.get(p) ?? digestAlone(p),
            // One script that fails to bundle fails the whole batch.
            () => digestAlone(p)
          )
        );
      }
      await Promise.all(pending.map((p) => digests.get(p)));
      log.info(
        `Computed bundle digests of ${pending.length} scripts in codebase ${
          codebase.relative_path
        } (${(performance.now() - startTime).toFixed(0)}ms)`
      );
    };

    const getDigest = async (scriptPath: string, forceTar?: boolean) => {
      let digest: string;
      if (codebase.customBundler) {
        digest = await getDirDigest();
      } else {
        const key = path.resolve(scriptPath);
        let pending = digests.get(key);
        if (!pending) {
          pending = digestAlone(scriptPath);
          digests.set(key, pending);
        }
        digest = await pending;
        log.debug(`Codebase ${codebase.relative_path}, ${scriptPath} digest: ${digest}`);
      }
      if (codebase.format == "esm") {
        digest += ".esm";
      }
      return forceTar || hasAssets ? digest + ".tar" : digest;
    };
    res.push({ ...codebase, getDigest, primeDigests });
  }

  return res;
}
