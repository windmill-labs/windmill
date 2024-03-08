const bo = await Bun.build({
  entrypoints: ["./main.ts"],
  outdir: "./out",
  plugins: [p],
  external: ["*"],
});

const fs = require("fs/promises");

const captureVersion =
  /^((?:\@[^\/\@]+\/[^\/\@]+)|(?:[^\/\@]+))(?:\@([^\/]+))?.*$/;

if (!bo.success) {
  bo.logs.forEach((l) => console.log(l));
  process.exit(1);
} else {
  let content = await fs.readFile("./out/main.js", { encoding: "utf8" });
  const imports = new Bun.Transpiler().scanImports(
    content.replaceAll("__require", "require")
  );

  const dependencies: Record<string, string[]> = {};
  for (const i of imports) {
    let [_, name, version] = i.path.match(captureVersion) ?? [];
    if (name == undefined) {
      throw Error("Unrecognized import: " + i.path);
    }
    if (name.startsWith("node:")) {
      continue;
    }
    let splitted = name.split("/");
    if (splitted.length > 2) {
      name = splitted.slice(0, 2).join("/");
    }
    if (version == undefined) {
      if (dependencies[name] == undefined) {
        dependencies[name] = [];
      }
    } else {
      if (dependencies[name] == undefined) {
        dependencies[name] = [version];
      } else {
        dependencies[name].push(version);
      }
    }
  }
  const resolvedDeps: Record<string, string> = {};
  for (const i in dependencies) {
    const versions = dependencies[i];
    resolvedDeps[i] =
      versions.length == 0
        ? "latest"
        : versions.length == 1
        ? versions[0]
        : reduceIntersect(versions);
  }
  await Bun.write(
    "./package.json",
    JSON.stringify({ dependencies: resolvedDeps }, null, 2)
  );

  function reduceIntersect(versions: string[]): string {
    const { intersect } = require("semver-intersect");
    return versions.reduce((a, b) => {
      return intersect(a, b);
    });
  }
}
