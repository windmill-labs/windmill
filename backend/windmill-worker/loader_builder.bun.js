try {
  await Bun.build({
    entrypoints: ["./main.ts"],
    outdir: "./out",
    plugins: [p],
    external: ["*"],
  });
} catch (err) {
  console.log(err);
  console.log("Failed to build bundle");
  process.exit(1);
}

const fs = require("fs/promises");

const captureVersion =
  /^((?:\@[^\/\@]+\/[^\/\@]+)|(?:[^\/\@]+))(?:\@([^\/]+))?.*$/;

import { semver } from "bun";

let content = await fs.readFile("./out/main.js", { encoding: "utf8" });
const imports = new Bun.Transpiler().scanImports(
  content.replaceAll("__require", "require")
);

const dependencies = {};
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
    } else if (!dependencies[name].includes(version)) {
      dependencies[name].push(version);
    }
  }
}
const resolvedDeps = {};
for (const i in dependencies) {
  const versions = dependencies[i];
  resolvedDeps[i] =
    versions.length == 0
      ? "latest"
      : versions.length == 1
      ? versions[0]
      : reduceIntersect(versions, i);
}
await Bun.write(
  "./package.json",
  JSON.stringify({ dependencies: resolvedDeps }, null, 2)
);

function reduceIntersect(versions, name) {
  console.log(`multiple versions detected for ${name}: ${versions.join(", ")}`);
  const regex = /^(?:\^|~|<|<=|>=|>)+/g;

  const r = versions
    .map((x) => [x, x.replace(regex, "")])
    .sort((a, b) => semver.order(a[1], b[1]))[0][0];
  console.log(`resolved to ${r} for ${name}`);
  return r;
}
