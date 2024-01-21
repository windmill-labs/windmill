const p = {
  name: "windmill-relative-resolver",
  async setup(build) {
    const { writeFileSync, readFileSync, mkdirSync } = await import("fs");
    const { dirname, resolve } = await import("node:path");

    const base_internal_url = "BASE_INTERNAL_URL".replace(
      "localhost",
      "127.0.0.1"
    );
    const w_id = "W_ID";
    const current_path = "CURRENT_PATH";

    build.onLoad({ filter: /.*\.url$/ }, async (args) => {
      const url = readFileSync(args.path, "utf8");
      const contents = await (
        await fetch(url, {
          method: "GET",
          headers: { Authorization: "Bearer TOKEN" },
        })
      ).text();
      return {
        contents,
        loader: "tsx",
      };
    });
    const cdir = resolve("./");
    const cdirNoPrivate = cdir.replace(/^\/private/, ""); // for macos
    const filter = new RegExp(
      `^(?!\\.\/main\\.ts)(?!${cdir}\/main\\.ts)(?!(?:/private)?${cdirNoPrivate}\/wrapper\\.ts).*\\.ts$`
    );
    build.onResolve({ filter }, (args) => {
      const file_path =
        args.importer == "./main.ts" || args.importer == resolve("./main.ts")
          ? current_path
          : args.importer.replace(cdir + "/", "");

      const isRelative = !args.path.startsWith("/");

      const url = isRelative
        ? `${base_internal_url}/api/w/${w_id}/scripts/RAW_GET_ENDPOINT/p/${file_path}/../${args.path}`
        : `${base_internal_url}/api/w/${w_id}/scripts/RAW_GET_ENDPOINT/p/${args.path}`;
      const file = isRelative
        ? resolve("./" + current_path + "/../" + args.path + ".url")
        : resolve("./" + args.path + ".url");
      mkdirSync(dirname(file), { recursive: true });
      writeFileSync(file, url);
      return {
        path: file,
      };
    });
  },
};
