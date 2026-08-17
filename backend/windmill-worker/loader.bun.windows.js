// Injected by backend: maps normalized paths to temp storage hashes (or null)
const TEMP_SCRIPT_REFS = TEMP_SCRIPT_REFS_PLACEHOLDER;

// Windows-specific bun loader that uses a virtual "windmill-url" namespace instead
// of writing .url files to disk. This avoids Windows path issues (backslashes in
// resolve(), 8.3 short filenames, drive letter prefixes). The virtual namespace
// approach is likely better on all fronts but we keep the original .url-file loader
// on Linux to avoid breaking back-compat.
const p = {
  name: "windmill-relative-resolver",
  async setup(build) {
    const { readFileSync } = await import("fs");
    const { resolve, dirname } = await import("node:path");

    const base_internal_url = "BASE_INTERNAL_URL".replace(
      "localhost",
      "127.0.0.1"
    );

    const w_id = "W_ID";
    const current_path = "CURRENT_PATH";
    const token = "TOKEN";

    const cdir = resolve("./");
    const cdirNoPrivate = cdir.replace(/^\/private/, ""); // for macos
    // Normalize path to forward slashes to match Bun's resolver output on Windows
    const cdirFwd = cdir.replace(/\\/g, "/");
    const cdirPosix = cdirFwd.replace(/^[a-zA-Z]:/, "");
    // Match either an already-normalized `.ts` specifier OR an *extensionless*
    // windmill relative/workspace import (`./`, `../`, `f/`, `/f/`, `u/`, `/u/`).
    // The extensionless branch is essential on Windows: the `main.ts` onLoad that
    // would rewrite `./mid` → `./mid.ts` only fires when its path filter matches
    // bun's resolver output, and the 8.3 short-name (`RUNNER~1`) vs canonical-path
    // mismatch makes that unreliable — so bare imports must be resolvable here
    // directly rather than depending on the rewrite. The `(?!.*\.[A-Za-z0-9]+$)`
    // guard keeps extension-bearing relative imports (a package's internal
    // `./foo.js`/`./x.json` requires) OUT of this resolver so they fall through
    // to bun's default resolver — a windmill script import is always `.ts` or
    // extensionless. Bare npm specifiers (no `./` prefix, not `f/`/`u/`) never
    // match either branch.
    const filterResolve = new RegExp(
      `^(?!\\.\/main\\.ts)(?!\\.\/_wm_)(?!${cdirFwd}\/main\\.ts)(?!${cdirFwd}\/_wm_)(?!${cdirPosix}\/main\\.ts)(?!${cdirPosix}\/_wm_)(?!(?:/private)?${cdirNoPrivate}\/wrapper\\.mjs)(?:.*\\.ts|(?:\\.\\.?\/|\/?f\/|\/?u\/)(?!.*\\.[A-Za-z0-9]+$).*)$`
    );

    // Match the entry main.ts against bun's forward-slash resolver output on
    // Windows — raw `cdir` carries backslashes that corrupt the regex, so the
    // onLoad below would never fire and extensionless relative imports in
    // main.ts would slip past filterResolve unrewritten.
    const filterLoad = new RegExp(`^(?:${cdirFwd}|${cdirPosix})\/main\\.ts$`);
    const transpiler = new Bun.Transpiler({
      loader: "ts",
    });

    function replaceRelativeImports(code) {
      const imports = transpiler.scanImports(code);
      for (const imp of imports) {
        if (imp.kind == "import-statement") {
          if (
            (imp.path.startsWith(".") ||
              imp.path.startsWith("/u/") ||
              imp.path.startsWith("/f/")) &&
            !imp.path.endsWith(".ts")
          ) {
            code = code.replaceAll(imp.path, imp.path + ".ts");
          }
        }
      }
      return {
        contents: code,
      };
    }

    function normalizePath(rawPath) {
      return rawPath.split("/").reduce((acc, seg) => {
        if (seg === "..") acc.pop();
        else if (seg !== "." && seg !== "") acc.push(seg);
        return acc;
      }, []).join("/");
    }

    // Resolve a windmill script import path relative to an importer path.
    // Bun on Windows may prefix args with "windmill-url:" or strip leading "/".
    function resolveWindmillImport(importerPath, importPath) {
      const path = importPath.replace(/^windmill-url:/, "").replace(/^\//, "");
      const isAbsolute = path.startsWith("f/") || path.startsWith("u/");
      const endExt = path.endsWith(".ts") ? "" : ".ts";
      const rawScriptPath = isAbsolute
        ? `${path}${endExt}`
        : `${importerPath}/../${path}${endExt}`;
      const normalized = normalizePath(rawScriptPath);
      // Look up temp script hash (keys are extensionless paths)
      const lookupPath = normalized.replace(/\.ts$/, "");
      const hash = TEMP_SCRIPT_REFS?.[lookupPath];
      // Encode hash in the path so onLoad can extract it and append to fetch URL
      const resolvedPath = hash ? `${normalized}?temp_script_hash=${hash}` : normalized;
      return { path: resolvedPath, namespace: "windmill-url" };
    }

    build.onLoad({ filter: filterLoad }, async (args) => {
      const code = readFileSync(args.path, "utf8");
      return replaceRelativeImports(code);
    });

    // Load windmill scripts by fetching from the API
    build.onLoad({ filter: /.*/, namespace: "windmill-url" }, async (args) => {
      // Extract temp_script_hash if embedded in the path by resolveWindmillImport
      const [scriptPath, queryString] = args.path.replace(/^windmill-url:/, "").split("?");
      const hashParam = queryString?.startsWith("temp_script_hash=")
        ? queryString.replace("temp_script_hash=", "")
        : undefined;
      const url = `${base_internal_url}/api/w/${w_id}/scripts/RAW_GET_ENDPOINT/p/${scriptPath}`
        + (hashParam ? `?temp_script_hash=${hashParam}` : "");
      const req = await fetch(url, {
        method: "GET",
        headers: {
          Authorization: "Bearer " + token,
        },
      });
      if (!req.ok) {
        throw new Error(
          `Failed to find relative import at ${url} (status ${req.status})`
        );
      }
      const contents = await req.text();
      return {
        contents: replaceRelativeImports(contents).contents,
        loader: "tsx",
      };
    });

    // Resolve windmill script imports from the file namespace (e.g. from main.ts)
    build.onResolve({ filter: filterResolve }, (args) => {
      const importerFwd = args.importer?.replace(/\\/g, "/") ?? "";
      // Let bun natively resolve any import originating INSIDE a dependency, so a
      // package's own relative requires (`./string.js`, extensionless `./foo`)
      // are never sent to the windmill resolver. Match `/node_modules/` anywhere
      // rather than a `cdir`-anchored prefix: on Windows `cdir` (canonical, e.g.
      // `runneradmin`) and the importer path (8.3 short name, e.g. `RUNNER~1`)
      // disagree, so a prefix check silently fails and dependency imports 404.
      if (importerFwd.includes("/node_modules/")) {
        return undefined;
      }
      // Check if the import resolves to a local module file (written by
      // write_module_files, which can nest files in subdirectories). Resolve
      // candidates against the importer's own directory — not the job root — so
      // a subdir module importing a sibling (`dir/a.ts` → `./b`) finds
      // `dir/b.ts` rather than a phantom `job_dir/b.ts`. For imports from
      // `main.ts` the importer dir IS the job root, so behavior is unchanged.
      // Module files carry a `.ts` extension, so also try the `.ts` variant of
      // a bare relative import before falling through to the remote resolver.
      if (args.path.startsWith(".")) {
        const importerDir = args.importer ? dirname(args.importer) : cdir;
        const candidates = args.path.endsWith(".ts")
          ? [args.path]
          : [args.path, args.path + ".ts"];
        for (const candidate of candidates) {
          const cwdPath = resolve(importerDir, candidate);
          try {
            readFileSync(cwdPath);
            return { path: cwdPath };
          } catch {}
        }
      }
      const isMainTs =
        args.importer == "./main.ts" || importerFwd.endsWith("/main.ts");
      const file_path = isMainTs
        ? current_path
        : importerFwd.replace(cdirFwd + "/", "");
      return resolveWindmillImport(file_path, args.path);
    });

    // Resolve nested imports from within windmill-url modules
    build.onResolve({ filter: /\.ts$/, namespace: "windmill-url" }, (args) => {
      // Strip any query string from the importer path before resolving
      const importer = args.importer.replace(/^windmill-url:/, "").split("?")[0];
      return resolveWindmillImport(importer, args.path);
    });
  },
};
