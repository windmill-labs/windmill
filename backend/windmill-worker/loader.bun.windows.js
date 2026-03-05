// Windows-specific bun loader that uses a virtual "windmill-url" namespace instead
// of writing .url files to disk. This avoids Windows path issues (backslashes in
// resolve(), 8.3 short filenames, drive letter prefixes). The virtual namespace
// approach is likely better on all fronts but we keep the original .url-file loader
// on Linux to avoid breaking back-compat.
const p = {
  name: "windmill-relative-resolver",
  async setup(build) {
    const { readFileSync } = await import("fs");
    const { resolve } = await import("node:path");

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
    const filterResolve = new RegExp(
      `^(?!\\.\/main\\.ts)(?!${cdirFwd}\/main\\.ts)(?!${cdirPosix}\/main\\.ts)(?!(?:/private)?${cdirNoPrivate}\/wrapper\\.mjs).*\\.ts$`
    );

    let cdirNodeModules = `${cdirFwd}/node_modules/`;

    const filterLoad = new RegExp(`^${cdir}\/main\\.ts$`);
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
      return { path: normalizePath(rawScriptPath), namespace: "windmill-url" };
    }

    build.onLoad({ filter: filterLoad }, async (args) => {
      const code = readFileSync(args.path, "utf8");
      return replaceRelativeImports(code);
    });

    // Load windmill scripts by fetching from the API
    build.onLoad({ filter: /.*/, namespace: "windmill-url" }, async (args) => {
      const path = args.path.replace(/^windmill-url:/, "");
      const url = `${base_internal_url}/api/w/${w_id}/scripts/RAW_GET_ENDPOINT/p/${path}`;
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
      if (importerFwd.startsWith(cdirNodeModules)) {
        return undefined;
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
      const importer = args.importer.replace(/^windmill-url:/, "");
      return resolveWindmillImport(importer, args.path);
    });
  },
};
