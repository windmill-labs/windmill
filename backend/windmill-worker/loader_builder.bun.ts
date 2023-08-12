const bo = await Bun.build({
  entrypoints: ["./main.ts"],
  outdir: "./out",
  plugins: [p],
  external: ["*"],
});

if (!bo.success) {
  bo.logs.forEach((l) => console.log(l));
  process.exit(1);
} else {
  let content = await Bun.file("./out/main.js").text();
  const imports = new Bun.Transpiler().scanImports(content);

  const { intersect } = require("semver-intersect");
  const dependencies: Record<string, string[]> = {};
  for (const i of imports) {
    let [name, version] = i.path.split("@");
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
