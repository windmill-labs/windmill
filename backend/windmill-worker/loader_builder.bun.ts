const bo = await Bun.build({
  entrypoints: ["./main.ts"],
  outdir: "./out",
  plugins: [p],
  external: ["*"],
});

const fs = require("fs/promises");

const captureVersion = /(^\@?[^\@]+)(?:\@(.+))?/;

if (!bo.success) {
  bo.logs.forEach((l) => console.log(l));
  process.exit(1);
} else {
  let content = await fs.readFile("./out/main.js", { encoding: "utf8" });
  const imports = new Bun.Transpiler().scanImports(
    content.replaceAll("__require", "require")
  );

  const { intersect } = require("semver-intersect");
  const dependencies: Record<string, string[]> = {};
  for (const i of imports) {
    let [_, name, version] = i.path.match(captureVersion) ?? [];
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
    resolvedDeps[i] =
      dependencies[i].length == 0
        ? "latest"
        : dependencies[i].reduce((a, b) => {
            return intersect(a, b);
          });
  }
  await Bun.write(
    "./package.json",
    JSON.stringify({ dependencies: resolvedDeps }, null, 2)
  );
}
