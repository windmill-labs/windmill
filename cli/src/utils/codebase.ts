import { Codebase, SyncOptions } from "../core/conf.ts";
import * as log from "../core/log.ts";
import { digestDir, generateHash, generateHashFromBuffer } from "./utils.ts";
import { withEsbuild } from "./esbuild_loader.ts";
import { readFile, realpath, stat } from "node:fs/promises";
import path from "node:path";

// A bare string is accepted, like `includes`/`excludes` in findCodebase.
function extraDigestPaths(codebase: Codebase): string[] {
  return ([] as string[]).concat(codebase.extra_digest_paths ?? []);
}

async function digestPath(p: string): Promise<string> {
  let s;
  try {
    s = await stat(p);
  } catch {
    throw new Error(
      `Codebase extra_digest_paths entry not found: ${p} (resolved to ${path.resolve(p)})`
    );
  }
  return s.isDirectory()
    ? await digestDir(p, "")
    : await generateHashFromBuffer(await readFile(p));
}

/** Bundle inputs that no codebase digest covers, so edits to them would not trigger a re-push. */
export function uncoveredBundleInputs(
  codebase: Codebase,
  entry: string,
  inputs: string[]
): string[] {
  const entryAbs = path.resolve(entry);
  const roots = [codebase.relative_path, ...extraDigestPaths(codebase)]
    .map((r) => path.resolve(r));
  return inputs.filter((i) => {
    // non-file namespaces ("<define:x>", "(disabled):x", "ns:path") and installed packages
    if (/^[<(]|^[a-z-]{2,}:/i.test(i) || i.includes("node_modules")) return false;
    const abs = path.resolve(i);
    return abs != entryAbs && !roots.some((r) => abs == r || abs.startsWith(r + path.sep));
  });
}

/** Whether a codebase digests each script from its own bundle inputs. A custom
 * bundler's inputs are unknown, so it keeps the directory digest. */
export function isPerScriptDigest(codebase: Codebase): boolean {
  return codebase.per_script_digest === true && !codebase.customBundler;
}

export type SyncCodebase = Codebase & {
  /** The `codebase` value a push stores for the script at `scriptPath`. */
  getDigest: (scriptPath: string, forceTar?: boolean) => Promise<string>;
  /** Computes the digests of many scripts in one esbuild pass. Only per-script
   * digest codebases do any work. */
  primeDigests: (scriptPaths: string[]) => Promise<void>;
};

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
  const hasher = new FileHasher();
  for (const codebase of options?.codebases ?? []) {
    const hasAssets =
      Array.isArray(codebase.assets) && codebase.assets.length > 0;
    const withSuffixes = (digest: string, forceTar?: boolean) => {
      if (codebase.format == "esm") digest += ".esm";
      return forceTar || hasAssets ? digest + ".tar" : digest;
    };

    if (!isPerScriptDigest(codebase)) {
      let dirDigest: Promise<string> | undefined;
      const getDigest = async (_scriptPath: string, forceTar?: boolean) => {
        if (!dirDigest) {
          dirDigest = (async () => {
            let digest = await digestDir(
              codebase.relative_path,
              JSON.stringify(codebase)
            );
            const extra = extraDigestPaths(codebase);
            if (extra.length > 0) {
              const hashes = await Promise.all(extra.map(digestPath));
              digest = await generateHash(digest + hashes.join(""));
            }
            log.info(
              `Codebase ${codebase.relative_path}, digest: ${withSuffixes(digest)}`
            );
            return digest;
          })();
        }
        return withSuffixes(await dirDigest, forceTar);
      };
      res.push({ ...codebase, getDigest, primeDigests: async () => {} });
      continue;
    }

    const digests = new Map<string, Promise<string>>();
    const primeDigests = async (scriptPaths: string[]) => {
      const missing = [
        ...new Set(scriptPaths.map((p) => path.resolve(p))),
      ].filter((p) => !digests.has(p));
      if (missing.length == 0) return;
      const computed = scriptDigests(codebase, missing, hasher);
      for (const p of missing) {
        const d = computed.then((m) => {
          const v = m.get(p)!;
          if (v instanceof Error) throw v;
          return v;
        });
        // A primed script nobody asks for must not fail the run.
        d.catch(() => {});
        digests.set(p, d);
      }
      await Promise.allSettled(missing.map((p) => digests.get(p)));
    };
    const getDigest = async (scriptPath: string, forceTar?: boolean) => {
      const abs = path.resolve(scriptPath);
      if (!digests.has(abs)) await primeDigests([abs]);
      return withSuffixes(await digests.get(abs)!, forceTar);
    };
    res.push({ ...codebase, getDigest, primeDigests });
  }

  return res;
}

/** Memoized content hashes, shared by every codebase of a run. */
class FileHasher {
  private hashes = new Map<string, Promise<string>>();
  private configs = new Map<string, Promise<string[]>>();
  private packages = new Map<string, Promise<string | undefined>>();
  private packageFields = new Map<string, Promise<string>>();

  /** The nearest package.json at or above `dir`. */
  packageFor(dir: string): Promise<string | undefined> {
    let p = this.packages.get(dir);
    if (!p) {
      p = (async () => {
        const here = path.join(dir, "package.json");
        if (await isFile(here)) return here;
        const parent = path.dirname(dir);
        return parent == dir ? undefined : await this.packageFor(parent);
      })();
      this.packages.set(dir, p);
    }
    return p;
  }

  /** A hash of the build-relevant fields of a package.json, or of the whole file
   * when it does not parse. */
  hashPackageFields(pkgPath: string): Promise<string> {
    let h = this.packageFields.get(pkgPath);
    if (!h) {
      h = (async () => {
        const pkg = await readJson(pkgPath);
        if (!pkg || typeof pkg != "object") return await this.hash(pkgPath);
        const fields = BUILD_PACKAGE_FIELDS.map((f) => [f, pkg[f]]);
        return await generateHash(JSON.stringify(fields));
      })();
      this.packageFields.set(pkgPath, h);
    }
    return h;
  }

  hash(absPath: string): Promise<string> {
    let h = this.hashes.get(absPath);
    if (!h) {
      h = readFile(absPath).then(generateHashFromBuffer);
      this.hashes.set(absPath, h);
    }
    return h;
  }

  /** The TS config esbuild applies to files in `dir`, as it picks it: the nearest
   * `tsconfig.json`, else `jsconfig.json`, in `dir` or an ancestor, followed by the
   * configs it `extends`. Ancestors it shadows are left out. */
  configsFor(dir: string): Promise<string[]> {
    let c = this.configs.get(dir);
    if (!c) {
      c = (async () => {
        for (const name of ["tsconfig.json", "jsconfig.json"]) {
          const p = path.join(dir, name);
          if (await isFile(p)) return await this.extendsChain(p, new Set());
        }
        const parent = path.dirname(dir);
        return parent == dir ? [] : await this.configsFor(parent);
      })();
      this.configs.set(dir, c);
    }
    return c;
  }

  private async extendsChain(config: string, seen: Set<string>): Promise<string[]> {
    if (seen.has(config)) return [];
    seen.add(config);
    const res = [config];
    for (const spec of tsconfigExtends(await readFile(config, "utf8"))) {
      const base = await resolveTsconfigExtends(spec, path.dirname(config));
      if (base) res.push(...(await this.extendsChain(base, seen)));
    }
    return res;
  }
}

const isFile = (p: string) => stat(p).then((s) => s.isFile(), () => false);

/** The `extends` of a tsconfig, which is JSON with comments and trailing commas.
 * An unparseable config extends nothing; its own content is still hashed. */
export function tsconfigExtends(text: string): string[] {
  text = text.replace(/^﻿/, "");
  let json = "";
  for (let i = 0; i < text.length; i++) {
    const ch = text[i];
    if (ch == '"') {
      const start = i;
      for (i++; i < text.length && text[i] != '"'; i++) if (text[i] == "\\") i++;
      json += text.slice(start, i + 1);
    } else if (ch == "/" && text[i + 1] == "/") {
      while (i < text.length && text[i] != "\n") i++;
      json += "\n";
    } else if (ch == "/" && text[i + 1] == "*") {
      i = text.indexOf("*/", i + 2);
      if (i < 0) break;
      i++;
    } else {
      json += ch;
    }
  }
  try {
    const ext = JSON.parse(json.replace(/,(\s*[}\]])/g, "$1"))?.extends;
    return ([] as unknown[]).concat(ext ?? []).filter((e): e is string => typeof e == "string");
  } catch {
    return [];
  }
}

async function readJson(p: string): Promise<any> {
  try {
    return JSON.parse((await readFile(p, "utf8")).replace(/^﻿/, ""));
  } catch {
    return undefined;
  }
}

/**
 * Resolves an `extends` as esbuild 0.28 does (`parseTSConfigFromSource` in its
 * resolver). A path is relative to the config, with `.json` appended only when it
 * lacks it. A package is looked up in each `node_modules` above the config: through
 * its `exports` under the `require` conditions, else at the path its `tsconfig`
 * field names or at the subpath, as `<p>/tsconfig.json`, `<p>` or `<p>.json`.
 */
async function resolveTsconfigExtends(spec: string, dir: string): Promise<string | undefined> {
  const isPackage = !/^\.{1,2}(\/|\\|$)/.test(spec) && !path.isAbsolute(spec);
  if (!isPackage) {
    let p = spec == "." || spec == ".." ? spec + "/tsconfig.json" : spec;
    p = path.resolve(dir, p);
    if (await isFile(p)) return p;
    return !p.endsWith(".json") && (await isFile(p + ".json")) ? p + ".json" : undefined;
  }
  const parts = spec.split("/");
  const nameLen = spec.startsWith("@") ? 2 : 1;
  const name = parts.slice(0, nameLen).join("/");
  const subpath = parts.slice(nameLen).join("/");
  for (let d = dir; ; d = path.dirname(d)) {
    if (path.basename(d) != "node_modules") {
      let join = path.join(d, "node_modules", spec);
      const pkgDir = path.join(d, "node_modules", name);
      const pkg = await readJson(path.join(pkgDir, "package.json"));
      if (pkg && typeof pkg == "object") {
        if (typeof pkg.tsconfig == "string" && pkg.tsconfig != "") {
          join = path.resolve(pkgDir, pkg.tsconfig);
        }
        const target = exportsTarget(pkg.exports, subpath ? "./" + subpath : ".");
        if (target != undefined && (await isFile(path.join(pkgDir, target)))) {
          return path.join(pkgDir, target);
        }
      }
      for (const c of [path.join(join, "tsconfig.json"), join, join + ".json"]) {
        if (await isFile(c)) return c;
      }
    }
    if (path.dirname(d) == d) return undefined;
  }
}

/** The conditions esbuild resolves a tsconfig `extends` through `exports` with, for
 * `platform: "node"` and no custom `conditions`. */
const TSCONFIG_EXPORTS_CONDITIONS = new Set(["require", "node", "default"]);

/** The file a package's `exports` maps `key` ("." or "./sub") to under
 * {@link TSCONFIG_EXPORTS_CONDITIONS}, following node's algorithm: an exact subpath
 * first, else the `*` pattern with the longest prefix; within a conditions object,
 * the first key that is an active condition. */
export function exportsTarget(exports: unknown, key: string): string | undefined {
  const pick = (v: unknown): string | undefined => {
    if (typeof v == "string") return v;
    if (Array.isArray(v)) {
      for (const x of v) {
        const r = pick(x);
        if (r != undefined) return r;
      }
      return undefined;
    }
    if (v && typeof v == "object") {
      for (const [cond, x] of Object.entries(v)) {
        if (TSCONFIG_EXPORTS_CONDITIONS.has(cond)) return pick(x);
      }
    }
    return undefined;
  };
  if (exports == undefined || exports === null) return undefined;
  const isSubpathMap =
    typeof exports == "object" &&
    !Array.isArray(exports) &&
    Object.keys(exports).some((k) => k.startsWith("."));
  if (!isSubpathMap) return key == "." ? pick(exports) : undefined;
  const map = exports as Record<string, unknown>;
  if (key in map && !key.includes("*")) return pick(map[key]);
  let best: { pattern: string; pre: string; post: string } | undefined;
  for (const pattern of Object.keys(map)) {
    const star = pattern.indexOf("*");
    if (star < 0 || pattern.indexOf("*", star + 1) >= 0) continue;
    const pre = pattern.slice(0, star);
    const post = pattern.slice(star + 1);
    if (!key.startsWith(pre) || key == pre) continue;
    if (post && (!key.endsWith(post) || key.length < pattern.length)) continue;
    if (
      !best ||
      pre.length > best.pre.length ||
      (pre.length == best.pre.length && pattern.length > best.pattern.length)
    ) {
      best = { pattern, pre, post };
    }
  }
  if (!best) return undefined;
  const t = pick(map[best.pattern]);
  return t?.replaceAll("*", key.slice(best.pre.length, key.length - best.post.length));
}

/** The fields of a package.json that change how esbuild compiles or tree-shakes the
 * files below it without changing which files it bundles. Hashed instead of the
 * whole file, so adding a dependency does not redeploy every script; a dependency's
 * new version shows up in its bundled files. */
const BUILD_PACKAGE_FIELDS = ["sideEffects", "type", "main", "module", "browser", "exports", "imports"];

/** A file input of an esbuild metafile, as opposed to "<define:x>", "(disabled):x"
 * or a plugin namespace. */
function isFileInput(key: string): boolean {
  return !/^[<(]|^[a-z-]{2,}:/i.test(key);
}

const toPosix = (p: string) => p.split(path.sep).join("/");

/**
 * Per-script digests: each covers the files the script's bundle is built from
 * (the transitive imports esbuild resolves, `node_modules` included, by path and
 * content), the TS configs and package.json build fields that apply to them, the
 * codebase's assets and
 * `extra_digest_paths`, and the codebase config.
 *
 * It follows the import graph rather than the bundle output: an output's input list
 * drops files whose values are inlined (TS enums), and hashing output would make
 * every digest depend on esbuild's code generation. Paths are relative to the
 * working directory, the sync checkout root, so a script digests the same alone and
 * in a batch, on any machine.
 */
async function scriptDigests(
  codebase: Codebase,
  scriptPaths: string[],
  hasher: FileHasher
): Promise<Map<string, string | Error>> {
  const cwd = await realpath(process.cwd());
  const parts = [JSON.stringify(codebase)];
  for (const p of extraDigestPaths(codebase)) parts.push(await digestPath(p));
  for (const a of codebase.assets ?? []) {
    parts.push(`${a.to}\0${await hasher.hash(path.resolve(a.from))}`);
  }
  const shared = parts.join("\n");

  const closures = await importClosures(codebase, scriptPaths, cwd);
  const res = new Map<string, string | Error>();
  await Promise.all(
    scriptPaths.map(async (script) => {
      const files = closures.get(script)!;
      if (files instanceof Error) {
        res.set(script, files);
        return;
      }
      const configs = new Set<string>();
      const packages = new Set<string>();
      for (const f of files) {
        const pkg = await hasher.packageFor(path.dirname(f));
        if (pkg) packages.add(pkg);
        if (f.split(path.sep).includes("node_modules")) continue;
        for (const c of await hasher.configsFor(path.dirname(f))) {
          configs.add(c);
        }
      }
      const rel = (f: string) => toPosix(path.relative(cwd, f));
      const entries = await Promise.all([
        ...[...files, ...configs].map(async (f) => `${rel(f)}\0${await hasher.hash(f)}`),
        ...[...packages].map(
          async (p) => `${rel(p)}\0fields\0${await hasher.hashPackageFields(p)}`
        ),
      ]);
      entries.sort();
      res.set(
        script,
        await generateHash(shared + "\n" + entries.join("\n"))
      );
    })
  );
  return res;
}

/** Each script's bundle inputs as real absolute paths, the script itself included.
 * esbuild reports real paths, so `cwd` and the entries are resolved the same way: a
 * checkout under a symlink would otherwise find no inputs for its scripts. */
async function importClosures(
  codebase: Codebase,
  scriptPaths: string[],
  cwd: string
): Promise<Map<string, Set<string> | Error>> {
  const res = new Map<string, Set<string> | Error>();
  const entries = new Map<string, string>();
  for (const s of scriptPaths) {
    try {
      entries.set(s, await realpath(s));
    } catch (e) {
      res.set(s, bundleError(s, e));
    }
  }
  const inject = await Promise.all(
    (codebase.inject ?? []).map((p) => realpath(path.resolve(p)))
  );
  const graph = async (entries: string[], batched: boolean) => {
    const out = await withEsbuild((esbuild) =>
      esbuild.build({
        ...graphBuildOptions(codebase, batched),
        entryPoints: entries,
        absWorkingDir: cwd,
      })
    );
    const imports = new Map<string, string[]>();
    for (const [key, input] of Object.entries(out.metafile?.inputs ?? {})) {
      if (!isFileInput(key)) continue;
      imports.set(
        path.resolve(cwd, key),
        input.imports
          .filter((i) => !i.external && isFileInput(i.path))
          .map((i) => path.resolve(cwd, i.path))
      );
    }
    return imports;
  };
  const closure = (imports: Map<string, string[]>, script: string) => {
    if (!imports.has(script)) {
      throw new Error(`esbuild reported no inputs for ${script}`);
    }
    const seen = new Set<string>();
    const todo = [script, ...inject];
    while (todo.length > 0) {
      const f = todo.pop()!;
      if (seen.has(f)) continue;
      seen.add(f);
      todo.push(...(imports.get(f) ?? []));
    }
    return seen;
  };

  if (entries.size > 1) {
    try {
      const imports = await graph([...entries.values()], true);
      for (const [s, real] of entries) res.set(s, closure(imports, real));
      return res;
    } catch (e) {
      const reason = (e instanceof Error ? e.message : String(e)).split("\n")[0];
      log.info(
        `Codebase digest: the batched build failed (${reason}), digesting ${entries.size} scripts one by one`
      );
    }
  }
  // One script that does not bundle fails the whole batch; only it should fail.
  const queue = [...entries];
  await Promise.all(
    Array.from({ length: Math.min(8, queue.length) }, async () => {
      for (let q = queue.shift(); q != undefined; q = queue.shift()) {
        const [s, real] = q;
        try {
          res.set(s, closure(await graph([real], false), real));
        } catch (e) {
          res.set(s, bundleError(s, e));
        }
      }
    })
  );
  return res;
}

function bundleError(script: string, e: unknown): Error {
  const msg = e instanceof Error ? e.message : String(e);
  return new Error(
    `Cannot compute the codebase digest of ${toPosix(path.relative(process.cwd(), script))}: bundling it failed. ` +
      `With per_script_digest every codebase script must bundle for a push to compare it; fix the script or exclude it from the codebase.\n${msg}`
  );
}

/**
 * The esbuild options of the digest pass. Only the options that decide which files
 * are bundled matter (`external`, `inject`, `define`, which can drop a dead
 * `require`, `loader`, `platform`, `packages`); the output is discarded. A batch is
 * built with code splitting so that code shared by many scripts is generated once.
 */
function graphBuildOptions(codebase: Codebase, batched: boolean) {
  return {
    bundle: true,
    write: false,
    metafile: true,
    logLevel: "silent" as const,
    outdir: "/",
    platform: "node" as const,
    packages: "bundle" as const,
    external: codebase.external,
    inject: codebase.inject,
    define: codebase.define,
    loader: codebase.loader ?? { ".node": "file" },
    target: "esnext",
    ...(batched
      ? { format: "esm" as const, splitting: true }
      : { format: codebase.format ?? "cjs" }),
  };
}
