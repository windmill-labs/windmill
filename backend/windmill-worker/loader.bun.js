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
    // On Windows, normalize path to forward slashes to match Bun's resolver output
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

    build.onLoad({ filter: filterLoad }, async (args) => {
      const code = readFileSync(args.path, "utf8");
      return replaceRelativeImports(code);
    });

    // Load windmill scripts by fetching from the API
    build.onLoad({ filter: /.*/, namespace: "windmill-url" }, async (args) => {
      const url = `${base_internal_url}/api/w/${w_id}/scripts/RAW_GET_ENDPOINT/p/${args.path}`;
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
      const file_path =
        args.importer == "./main.ts" || importerFwd == cdirFwd + "/main.ts"
          ? current_path
          : importerFwd.replace(cdirFwd + "/", "");

      const isRelative = !args.path.startsWith("/");

      let endExt = args.path.endsWith(".ts") ? "" : ".ts";
      const rawScriptPath = isRelative
        ? `${file_path}/../${args.path}${endExt}`
        : `${args.path}${endExt}`;
      return {
        path: normalizePath(rawScriptPath),
        namespace: "windmill-url",
      };
    });

    // Resolve nested imports from within windmill-url modules
    build.onResolve({ filter: /\.ts$/, namespace: "windmill-url" }, (args) => {
      const isRelative = !args.path.startsWith("/");
      let endExt = args.path.endsWith(".ts") ? "" : ".ts";
      const rawScriptPath = isRelative
        ? `${args.importer}/../${args.path}${endExt}`
        : `${args.path}${endExt}`;
      return {
        path: normalizePath(rawScriptPath),
        namespace: "windmill-url",
      };
    });
  },
};
