import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import process from "node:process";
import { createGunzip } from "node:zlib";
import { Readable } from "node:stream";
import { pathToFileURL } from "node:url";
import * as tar from "tar-stream";
import * as log from "../core/log.ts";

// esbuild splits into a JS host package and a per-platform native binary
// (@esbuild/<platform>). They must be the same version. A broken or incremental
// install can leave the on-disk binary at a different version than the pinned
// host, which crashes service start with
//   Cannot start service: Host version "X" does not match binary version "Y"
// The running code can't fix what npm/bun put on disk, so when that happens we
// fall back to esbuild-wasm, whose binary is a single version-pinned .wasm. To
// keep that 14MB out of every CLI install, the esbuild-wasm package is not a
// dependency: it is downloaded once and cached on disk, on the fallback path
// only. We download the whole package (not just the .wasm) because esbuild-wasm
// reads the app's files from disk by spawning `node bin/esbuild`, which needs
// bin/esbuild + esbuild.wasm + wasm_exec*.js co-located on disk.

type Esbuild = typeof import("esbuild");

// Version to fall back to if the native host's version can't be read. Keep in
// sync with the "esbuild" pin in cli/package.json.
const FALLBACK_VERSION = "0.28.0";

let cached: Esbuild | undefined;
let inFlight: Promise<Esbuild> | undefined;
// Distinguishes concurrent extraction temp dirs within a process.
let extractCounter = 0;

/**
 * Returns a working esbuild module, preferring the native binary and falling
 * back to esbuild-wasm only when the native host/binary versions don't match.
 * Memoized for the process: concurrent first callers (e.g. a parallel
 * `wmill sync push`) share one probe/download instead of each running their own.
 */
export function getEsbuild(): Promise<Esbuild> {
  if (cached) return Promise.resolve(cached);
  if (inFlight) return inFlight;
  inFlight = acquireEsbuild()
    .then((esbuild) => {
      cached = esbuild;
      return esbuild;
    })
    .finally(() => {
      inFlight = undefined;
    });
  return inFlight;
}

async function acquireEsbuild(): Promise<Esbuild> {
  // Escape hatch: skip native entirely (e.g. a host known to have a broken
  // install, or to exercise the fallback path).
  if (process.env.WINDMILL_FORCE_ESBUILD_WASM) {
    return loadWasmEsbuild(await nativeHostVersion());
  }

  try {
    const esbuild = await import("esbuild");
    // The native service only starts on the first call; force it with the most
    // trivial op so any breakage (host/binary version mismatch, a dead service)
    // surfaces now rather than mid-build. The mismatch detail is printed to the
    // child's stderr while the thrown error is generic ("service was stopped"),
    // so we fall back on ANY smoke-test failure rather than matching a string.
    await esbuild.transform("");
    return esbuild;
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    log.warn(
      `native esbuild is not usable; falling back to esbuild-wasm (${msg.trim()})`
    );
  }

  return loadWasmEsbuild(await nativeHostVersion());
}

/**
 * Stops the esbuild service (native or wasm — both spawn a child process) so the
 * process can exit. Safe to call repeatedly; the service restarts lazily on the
 * next build.
 */
export async function stopEsbuild(): Promise<void> {
  await cached?.stop();
}

async function nativeHostVersion(): Promise<string> {
  try {
    return (await import("esbuild")).version ?? FALLBACK_VERSION;
  } catch {
    return FALLBACK_VERSION;
  }
}

async function loadWasmEsbuild(version: string): Promise<Esbuild> {
  const pkgDir = await ensureWasmPackage(version);
  const mainJs = path.join(pkgDir, "lib", "main.js");
  // The Node build (lib/main.js) reads app files from disk by spawning
  // `node bin/esbuild`, so it works with on-disk entry points and node_modules,
  // unlike the browser build.
  return (await import(pathToFileURL(mainJs).href)) as unknown as Esbuild;
}

/**
 * Returns a directory containing an extracted esbuild-wasm package (with
 * lib/main.js). Uses an explicit override, then an on-disk cache, then downloads
 * and extracts the npm tarball.
 */
async function ensureWasmPackage(version: string): Promise<string> {
  // Explicit local override wins (air-gapped / self-hosted workers): a path to
  // an already-extracted esbuild-wasm package directory.
  const override = process.env.WINDMILL_ESBUILD_WASM_PATH;
  if (override) return override;

  const destDir = path.join(cacheDir(), `esbuild-wasm-${version}`);
  if (fs.existsSync(path.join(destDir, "lib", "main.js"))) {
    return destDir;
  }

  const url = process.env.WINDMILL_ESBUILD_WASM_URL ??
    `https://registry.npmjs.org/esbuild-wasm/-/esbuild-wasm-${version}.tgz`;
  log.info(`Downloading esbuild-wasm@${version} from ${url} ...`);
  const res = await fetch(url);
  if (!res.ok || !res.body) {
    throw new Error(
      `Failed to download esbuild-wasm@${version} (${res.status} ${res.statusText}). ` +
        `Set WINDMILL_ESBUILD_WASM_PATH to an extracted esbuild-wasm package dir, ` +
        `point WINDMILL_ESBUILD_WASM_URL at a reachable tarball, or repair the native esbuild install.`
    );
  }

  // Extract to a unique temp dir and rename into place so a crash or a
  // concurrent writer can't leave a half-extracted package behind, and so two
  // extractions never share an in-progress directory.
  const tmpDir = `${destDir}.${process.pid}.${extractCounter++}.tmp`;
  fs.rmSync(tmpDir, { recursive: true, force: true });
  await extractTarball(res.body, tmpDir);
  if (!fs.existsSync(path.join(tmpDir, "lib", "main.js"))) {
    fs.rmSync(tmpDir, { recursive: true, force: true });
    throw new Error(`esbuild-wasm@${version} tarball did not contain lib/main.js`);
  }
  try {
    fs.renameSync(tmpDir, destDir);
  } catch {
    // Another process won the race, or rename across devices failed; clean up
    // and let the existsSync check below decide whether the cache is usable.
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
  if (!fs.existsSync(path.join(destDir, "lib", "main.js"))) {
    throw new Error(`Failed to cache esbuild-wasm@${version} at ${destDir}`);
  }
  return destDir;
}

/**
 * Resolves a tar entry to an absolute path inside destDir, stripping the leading
 * "package/" component that npm tarballs use. Returns null if the entry would
 * escape destDir (tar-slip), since WINDMILL_ESBUILD_WASM_URL allows untrusted
 * tarball sources.
 */
export function resolveTarEntryPath(
  destDir: string,
  entryName: string
): string | null {
  const rel = entryName.replace(/^[^/]+\//, "");
  const root = path.resolve(destDir);
  const outPath = path.resolve(root, rel);
  if (outPath !== root && !outPath.startsWith(root + path.sep)) {
    return null;
  }
  return outPath;
}

// Extracts an npm tarball (gzipped tar) into destDir, stripping the leading
// "package/" path component that npm tarballs use.
async function extractTarball(
  body: ReadableStream<Uint8Array>,
  destDir: string
): Promise<void> {
  const extract = tar.extract();
  extract.on("entry", (header, stream, next) => {
    if (header.type !== "file") {
      stream.resume();
      stream.on("end", next);
      return;
    }
    const outPath = resolveTarEntryPath(destDir, header.name);
    if (!outPath) {
      // Reject tar-slip entries that would write outside the cache dir.
      stream.resume();
      stream.on("end", () =>
        next(new Error(`unsafe path in esbuild-wasm tarball: ${header.name}`))
      );
      return;
    }
    fs.mkdirSync(path.dirname(outPath), { recursive: true });
    const ws = fs.createWriteStream(outPath, { mode: header.mode ?? 0o644 });
    stream.pipe(ws);
    ws.on("finish", next);
    ws.on("error", next);
    stream.on("error", next);
  });

  await new Promise<void>((resolve, reject) => {
    extract.on("finish", resolve);
    extract.on("error", reject);
    Readable.fromWeb(body as unknown as Parameters<typeof Readable.fromWeb>[0])
      .pipe(createGunzip())
      .on("error", reject)
      .pipe(extract)
      .on("error", reject);
  });
}

function cacheDir(): string {
  const explicit = process.env.WINDMILL_CACHE_DIR;
  if (explicit) return explicit;
  const xdg = process.env.XDG_CACHE_HOME;
  if (xdg) return path.join(xdg, "windmill");
  try {
    return path.join(os.homedir(), ".cache", "windmill");
  } catch {
    return path.join(os.tmpdir(), "windmill");
  }
}
