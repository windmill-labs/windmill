// Injected by backend: maps normalized paths to temp storage hashes (or null)
const TEMP_SCRIPT_REFS = TEMP_SCRIPT_REFS_PLACEHOLDER;

const p = {
  name: "windmill-relative-resolver",
  async setup(build) {
    const { writeFileSync, readFileSync, mkdirSync } = await import("fs");
    const { dirname, resolve, join } = await import("node:path");

    const base_internal_url = "BASE_INTERNAL_URL".replace(
      "localhost",
      "127.0.0.1"
    );

    const w_id = "W_ID";
    const current_path = "CURRENT_PATH";
    const token = "TOKEN";

    const cdir = resolve("./");
    const cdirNoPrivate = cdir.replace(/^\/private/, ""); // for macos
    // On Windows, normalize path to POSIX format to match args.path from Bun's resolver
    const cdirPosix = cdir.replace(/\\/g, "/").replace(/^[a-zA-Z]:/, "");
    const filterResolve = new RegExp(
      `^(?!\\.\/main\\.ts)(?!${cdir}\/main\\.ts)(?!${cdirPosix}\/main\\.ts)(?!(?:/private)?${cdirNoPrivate}\/wrapper\\.mjs).*\\.ts$`
    );

    let cdirNodeModules = `${cdir}/node_modules/`;

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

    build.onLoad({ filter: filterLoad }, async (args) => {
      const code = readFileSync(args.path, "utf8");
      return replaceRelativeImports(code);
    });

    build.onLoad({ filter: /.*\.url$/ }, async (args) => {
      const url = readFileSync(args.path, "utf8");
      const req = await fetch(url, {
        method: "GET",
        headers: {
          Authorization: "Bearer " + token,
        },
      });
      if (!req.ok) {
        throw new Error(
          `Failed to find relative import at ${url}`,
          req.statusText
        );
      }
      const contents = await req.text();
      return {
        contents: replaceRelativeImports(contents).contents,
        loader: "tsx",
      };
    });

    build.onResolve({ filter: filterResolve }, (args) => {
      if (args.importer?.startsWith(cdirNodeModules)) {
        return undefined;
      }

      // Check if the import resolves to a local module file (written by write_module_files).
      // Only check relative paths — absolute/bare specifiers should fall through to the
      // remote resolver, matching the Windows loader pattern.
      if (args.path.startsWith(".")) {
        const localPath = resolve(cdir, args.path);
        try {
          readFileSync(localPath);
          return { path: localPath };
        } catch {}
      }

      const file_path =
        args.importer == "./main.ts" || args.importer == resolve("./main.ts")
          ? current_path
          : args.importer.replace(cdir + "/", "");

      const isRelative = !args.path.startsWith("/");
      const endExt = args.path.endsWith(".ts") ? "" : ".ts";
      const pathNoExt = args.path.replace(/\.ts$/, "");

      // Lookup temp script hash
      const normalized = (isRelative ? join(dirname(file_path), pathNoExt) : pathNoExt.slice(1)).replace(/\\/g, "/");
      const hash = TEMP_SCRIPT_REFS?.[normalized];

      const url = (isRelative
        ? `${base_internal_url}/api/w/${w_id}/scripts/raw_unpinned/p/${file_path}/../${args.path}${endExt}`
        : `${base_internal_url}/api/w/${w_id}/scripts/raw_unpinned/p/${args.path}${endExt}`
      ) + (hash ? `?temp_script_hash=${hash}` : "");
      const file = isRelative
        ? resolve("./" + file_path + "/../" + args.path + ".url")
        : resolve("./" + args.path + ".url");
      mkdirSync(dirname(file), { recursive: true });
      writeFileSync(file, url);
      return {
        path: file,
      };
    });
  },
};
