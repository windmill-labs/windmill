import { plugin } from "bun";

plugin({
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
    build.onResolve({ filter: /(?!\.\/main\.ts)\..*\.ts$/ }, (args) => {
      const cdir = resolve("./");
      const file_path =
        args.importer == resolve("./main.ts")
          ? current_path
          : args.importer.replace(cdir + "/", "");

      const url = `${base_internal_url}/api/w/${w_id}/scripts/raw/p/${file_path}/../${args.path}`;
      const file = resolve("./" + current_path + "/../" + args.path + ".url");
      mkdirSync(dirname(file), { recursive: true });
      writeFileSync(file, url);
      return {
        path: file,
      };
    });
  },
});
