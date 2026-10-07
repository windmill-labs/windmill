// Node's ESM loader only exposes the named exports of a CommonJS package that
// cjs-module-lexer can detect statically. lodash & co. build `module.exports` at
// runtime, so `import { chunk } from "lodash"` fails at instantiation and
// `import * as _ from "lodash"` yields a namespace holding nothing but `default`.
// npm packages stay external to the bundle, so Bun emits those import statements
// verbatim and node hits the limitation; rewrite them to a namespace import plus
// a lookup that falls back to `default` (i.e. `module.exports`).
//
// Only packages node loads as CommonJS are rewritten: the rewrite turns named
// imports into snapshots, which would freeze an ESM export its package mutates
// after evaluation.

import { readFileSync } from "node:fs";
import { dirname, extname, join } from "node:path";
import { fileURLToPath } from "node:url";

const WM_IDENT = "[A-Za-z_$][A-Za-z0-9_$]*";
const WM_NS_CLAUSE = `\\*\\s*as\\s+${WM_IDENT}`;
const WM_NAMED_CLAUSE = "\\{[^{}]*\\}";
const WM_CLAUSE = `(?:${WM_NS_CLAUSE}|${WM_NAMED_CLAUSE}|${WM_IDENT}(?:\\s*,\\s*(?:${WM_NS_CLAUSE}|${WM_NAMED_CLAUSE}))?)`;
const WM_ATTRS = "\\s*(?:with|assert)\\s*\\{[^{}]*\\}";
const WM_IMPORT = `(^|[;}\\n])import\\s*(?:(${WM_CLAUSE})\\s*from\\s*)?(?:"([^"\\n]*)"|'([^'\\n]*)')(${WM_ATTRS})?`;

function wmRewriteExternalImports(code, externals, jobDir, nodePath) {
  if (!externals || externals.length === 0) {
    return code;
  }
  const isExternal = (spec) =>
    externals.some((name) => spec === name || spec.startsWith(name + "/"));

  // Import statements are found on a copy whose literals are blanked out, so
  // that generated code holding an import statement in a string is not touched.
  // Blanking preserves offsets, but mis-pairing a literal's boundaries can still
  // expose a string's contents as code, so the parser cross-check below is what
  // makes the rewrite safe — it is load-bearing, not a belt-and-braces extra.
  const masked = wmMaskLiterals(code);
  const anchored = new RegExp(WM_IMPORT);
  const matches = [];
  for (const hit of masked.matchAll(new RegExp(WM_IMPORT, "g"))) {
    const m = code.slice(hit.index, hit.index + hit[0].length).match(anchored);
    if (m === null || m.index !== 0) {
      continue;
    }
    const spec = m[3] !== undefined ? m[3] : m[4];
    if (isExternal(spec)) {
      matches.push({
        at: hit.index,
        len: hit[0].length,
        lead: m[1],
        clause: m[2],
        spec,
        attrs: m[5] ?? "",
      });
    }
  }
  if (matches.length === 0) {
    return code;
  }

  const commonjs = wmCommonJsSpecs(
    [...new Set(matches.map((m) => m.spec))],
    jobDir,
    nodePath
  );
  const found = matches.filter((m) => commonjs.has(m.spec));
  if (found.length === 0) {
    return code;
  }

  // If the statements found are not the ones the parser sees, the masking
  // mis-paired a literal somewhere: leave the bundle alone rather than corrupt it.
  const counts = new Map();
  for (const f of found) {
    counts.set(f.spec, (counts.get(f.spec) ?? 0) + 1);
  }
  for (const imp of new Bun.Transpiler({ loader: "js" }).scanImports(code)) {
    if (imp.kind === "import-statement" && counts.has(imp.path)) {
      counts.set(imp.path, counts.get(imp.path) - 1);
    }
  }
  for (const [spec, left] of counts) {
    if (left !== 0) {
      console.log(
        `Skipping CommonJS interop rewrite of "${spec}": import statements found do not match the parsed ones`
      );
      return code;
    }
  }

  let prefix = "__wm_ext";
  while (code.includes(prefix)) {
    prefix += "_";
  }
  const getHelper = `${prefix}Get`;
  const nsHelper = `${prefix}Ns`;
  let out = "";
  let cursor = 0;
  let count = 0;
  for (const f of found) {
    const parts = f.clause === undefined ? null : wmParseImportClause(f.clause);
    // A default-only import already resolves to `module.exports` under node.
    if (parts === null || (parts.named.length === 0 && parts.ns === null)) {
      continue;
    }
    const ns = `${prefix}${count++}`;
    const decls = [];
    if (parts.def !== null) {
      decls.push(`${parts.def}=${ns}.default`);
    }
    if (parts.ns !== null) {
      decls.push(`${parts.ns}=${nsHelper}(${ns})`);
    }
    for (const [imported, local] of parts.named) {
      decls.push(
        `${local}=${getHelper}(${ns},${JSON.stringify(imported)},${JSON.stringify(f.spec)})`
      );
    }
    out +=
      code.slice(cursor, f.at) +
      `${f.lead}import*as ${ns} from${JSON.stringify(f.spec)}${f.attrs};const ${decls.join(",")};`;
    cursor = f.at + f.len;
  }
  if (count === 0) {
    return code;
  }

  return (
    `var ${getHelper}=(n,k,s)=>{if(k in n)return n[k];let d=Object(n.default);if(k in d)return d[k];` +
    "throw new SyntaxError(`The requested module '${s}' does not provide an export named '${k}'`)};" +
    `var ${nsHelper}=(n)=>{let d=n.default;if(d==null||typeof d!=="object"&&typeof d!=="function")return n;` +
    `let t={},a=(o,k)=>Object.defineProperty(t,k,{get:()=>o[k],enumerable:!0,configurable:!0});` +
    `for(let k of Object.keys(d))a(d,k);for(let k of Object.keys(n))a(n,k);` +
    `return new Proxy(t,{get:(x,k,r)=>k in x?Reflect.get(x,k,r):d[k],has:(x,k)=>k in x||k in d})};` +
    out +
    code.slice(cursor)
  );
}

// Node runs the bundle, and conditional exports can hand it a different file
// than they hand bun, so ask node itself where each specifier resolves. Bun's
// resolver is only the fallback for a node too old for `import.meta.resolve`.
function wmCommonJsSpecs(specs, jobDir, nodePath) {
  const commonjs = new Set();
  let resolved = null;
  const probe =
    "if(typeof import.meta.resolve!=='function')process.exit(3);const out={};" +
    "for(const s of JSON.parse(process.argv[1])){try{out[s]=import.meta.resolve(s)}catch(e){out[s]=null}}" +
    "console.log(JSON.stringify(out))";
  try {
    const run = Bun.spawnSync([nodePath, "--input-type=module", "-e", probe, JSON.stringify(specs)], {
      cwd: jobDir,
    });
    if (run.success) {
      resolved = JSON.parse(run.stdout.toString());
    }
  } catch (err) {}
  for (const spec of specs) {
    let file;
    if (resolved === null) {
      try {
        file = Bun.resolveSync(spec, jobDir);
      } catch (err) {
        continue;
      }
    } else if (resolved[spec]?.startsWith("file:")) {
      file = fileURLToPath(resolved[spec]);
    } else {
      continue;
    }
    if (wmIsCommonJsFile(file)) {
      commonjs.add(spec);
    }
  }
  return commonjs;
}

// Node's own rule for a file's format: the extension decides, and `.js` — like an
// extensionless entry — follows the `type` of the closest package.json. Only the
// formats node loads through CommonJS qualify; a JSON module exposes `default`
// alone and must keep that shape.
function wmIsCommonJsFile(file) {
  const ext = extname(file);
  if (ext === ".cjs" || ext === ".node") {
    return true;
  }
  if (ext !== ".js" && ext !== "") {
    return false;
  }
  let dir = dirname(file);
  for (;;) {
    let pkg = null;
    try {
      pkg = JSON.parse(readFileSync(join(dir, "package.json"), "utf8"));
    } catch (err) {}
    if (pkg !== null) {
      return pkg.type !== "module";
    }
    const parent = dirname(dir);
    if (parent === dir) {
      return true;
    }
    dir = parent;
  }
}

function wmParseImportClause(clause) {
  let def = null;
  let ns = null;
  const named = [];
  let rest = clause.trim();

  if (!rest.startsWith("{") && !rest.startsWith("*")) {
    const m = rest.match(new RegExp(`^(${WM_IDENT})\\s*(?:,([\\s\\S]*))?$`));
    if (m === null) {
      return null;
    }
    def = m[1];
    rest = (m[2] ?? "").trim();
  }

  if (rest.startsWith("*")) {
    const m = rest.match(new RegExp(`^\\*\\s*as\\s+(${WM_IDENT})$`));
    if (m === null) {
      return null;
    }
    ns = m[1];
  } else if (rest.startsWith("{")) {
    const specifier = new RegExp(
      `^(${WM_IDENT}|"[^"]*"|'[^']*')(?:\\s+as\\s+(${WM_IDENT}))?$`
    );
    for (const part of rest.slice(1, rest.lastIndexOf("}")).split(",")) {
      const trimmed = part.trim();
      if (trimmed === "") {
        continue;
      }
      const m = trimmed.match(specifier);
      if (m === null) {
        return null;
      }
      const quoted = m[1].startsWith('"') || m[1].startsWith("'");
      if (quoted && m[2] === undefined) {
        return null;
      }
      named.push([quoted ? m[1].slice(1, -1) : m[1], m[2] ?? m[1]]);
    }
  } else if (rest !== "") {
    return null;
  }

  return { def, ns, named };
}

// Replaces the contents of every string, template, comment and regex literal
// with `x`, keeping the delimiters, every newline and the total length.
function wmMaskLiterals(code) {
  let out = "";
  let i = 0;
  let cursor = 0;
  let prev = "";
  const blank = (from, to) => {
    out += code.slice(cursor, from);
    for (const c of code.slice(from, to)) {
      out += c === "\n" ? "\n" : "x";
    }
    cursor = to;
  };
  while (i < code.length) {
    const c = code[i];
    if (c === '"' || c === "'" || c === "`") {
      const start = ++i;
      while (i < code.length) {
        if (code[i] === "\\") {
          i += 2;
        } else if (code[i] === c) {
          break;
        } else {
          i++;
        }
      }
      blank(start, Math.min(i, code.length));
      i++;
      prev = "'";
      continue;
    }
    if (c === "/" && code[i + 1] === "/") {
      const start = i + 2;
      while (i < code.length && code[i] !== "\n") {
        i++;
      }
      blank(start, i);
      continue;
    }
    if (c === "/" && code[i + 1] === "*") {
      const end = code.indexOf("*/", i + 2);
      const start = i + 2;
      i = end === -1 ? code.length : end + 2;
      blank(start, Math.max(start, i - 2));
      continue;
    }
    // `/` only starts a regex where an operand cannot stand; a wrong guess can
    // only make the masking hide more than it should, which the parser
    // cross-check catches.
    if (c === "/" && !/[A-Za-z0-9_$)\]]/.test(prev)) {
      const start = ++i;
      let inClass = false;
      while (i < code.length) {
        if (code[i] === "\\") {
          i += 2;
        } else if (code[i] === "[") {
          inClass = true;
          i++;
        } else if (code[i] === "]") {
          inClass = false;
          i++;
        } else if (code[i] === "\n" || (code[i] === "/" && !inClass)) {
          break;
        } else {
          i++;
        }
      }
      blank(start, Math.min(i, code.length));
      i++;
      prev = "'";
      continue;
    }
    if (!/\s/.test(c)) {
      prev = c;
    }
    i++;
  }
  return out + code.slice(cursor);
}
