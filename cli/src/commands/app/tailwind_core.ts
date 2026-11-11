/**
 * Tailwind CSS v4 for raw-app builds, independent of the bundler and of where
 * files live. The CLI's esbuild and the UI Builder's esbuild-wasm/rolldown
 * workers each supply a `TailwindHost`; this file is copied byte for byte into
 * windmill-code-ui-builder (`src/lib/tailwind_core.ts`), so keep it free of
 * Node and browser APIs and change both copies together.
 */

/** The `compile` export of the app's own `tailwindcss` (v4). */
export type TailwindCompile = (
  css: string,
  opts: {
    base: string;
    from?: string;
    loadStylesheet: (
      id: string,
      base: string,
    ) => Promise<{ path: string; base: string; content: string }>;
    loadModule: (
      id: string,
      base: string,
      resourceHint: "plugin" | "config",
    ) => Promise<{ path: string; base: string; module: unknown }>;
  },
) => Promise<{
  sources: { base: string; pattern: string; negated: boolean }[];
  root: null | "none" | { base: string; pattern: string };
  features: number;
  build(candidates: string[]): string;
}>;

/** Where `@source` (or `source(...)` on the import) points, before globbing. */
export interface TailwindSource {
  base: string;
  pattern: string;
}

export interface TailwindHost {
  /** Loaded on first use, so an app whose stylesheets never need it pays nothing. */
  loadCompile(): Promise<TailwindCompile>;
  loadStylesheet(
    id: string,
    base: string,
  ): Promise<{ path: string; base: string; content: string }>;
  loadModule(
    id: string,
    base: string,
    resourceHint: "plugin" | "config",
  ): Promise<{ path: string; base: string; module: unknown }>;
  /**
   * `url` as written in a stylesheet in directory `from`, made relative to
   * directory `to`, with forward slashes and a leading `./` or `../`.
   */
  rebaseUrl(url: string, from: string, to: string): string;
  /**
   * The files whose class names the stylesheet must cover: the app's sources
   * (and the shared `/ui` folder) when `includeDefault`, plus whatever each
   * `@source` names. `dirs` are the directories walked, for watch mode.
   */
  scan(
    sources: TailwindSource[],
    includeDefault: boolean,
  ): Promise<{ files: { path: string; content: string }[]; dirs: string[] }>;
}

export interface TailwindResult {
  css: string;
  /** Every file the output depends on: stylesheets read and sources scanned. */
  files: string[];
  dirs: string[];
}

// A stylesheet without any of these is plain CSS and is left to the bundler,
// which is what keeps a Tailwind app's other stylesheets (and every app without
// Tailwind) on the bundler's normal path.
const TAILWIND_DIRECTIVE =
  /@(?:import\s+(?:url\(\s*)?["']tailwindcss(?:["'/])|tailwind\b|theme\b|utility\b|apply\b|variant\b|custom-variant\b|source\b|plugin\b|config\b|reference\b)/;

export function usesTailwind(css: string): boolean {
  return TAILWIND_DIRECTIVE.test(css);
}

/** `"tailwindcss"` at major version 4 or later in a package.json's dependencies. */
export function declaresTailwindV4(packageJson: string | undefined): boolean {
  if (!packageJson) return false;
  let pkg: any;
  try {
    pkg = JSON.parse(packageJson);
  } catch {
    return false;
  }
  const spec = pkg?.dependencies?.tailwindcss ?? pkg?.devDependencies?.tailwindcss;
  if (typeof spec !== "string") return false;
  const major = spec.match(/\d+/);
  // A tag or URL ("latest", "next", a git link) has no major to read: latest is v4+.
  return major ? Number(major[0]) >= 4 : true;
}

export function installedTailwindMajor(packageJson: string | undefined): number | undefined {
  if (!packageJson) return undefined;
  try {
    const major = String(JSON.parse(packageJson)?.version ?? "").match(/^\d+/);
    return major ? Number(major[0]) : undefined;
  } catch {
    return undefined;
  }
}

const TRIM_START = /^[\s"'`{}()<>;,=]+/;
const TRIM_END = /[\s"'`{}()<>;,=]+$/;

function addCandidate(token: string, out: Set<string>) {
  let t = token.replace(TRIM_START, "").replace(TRIM_END, "");
  if (t.startsWith("class:")) t = t.slice("class:".length);
  if (t.length === 0 || t.length > 300) return;
  out.add(t);
  // An object key (`clsx({ hidden: open })`, `{hidden:open}`) carries the
  // colon, or the value, of the key/value pair it belongs to.
  const key = t.match(/^([\w-]+):(?:[A-Za-z_$!][\w$.!]*)?$/);
  if (key) out.add(key[1]);
}

/** `className="after:content-['x']"` -> `className=`, `after:content-['x']`. */
function splitQuotesOutsideBrackets(token: string): string[] {
  const parts: string[] = [];
  let depth = 0;
  let start = 0;
  for (let i = 0; i < token.length; i++) {
    const c = token[i];
    if (c === "[") depth++;
    else if (c === "]") depth = Math.max(0, depth - 1);
    else if (depth === 0 && (c === '"' || c === "'" || c === "`")) {
      parts.push(token.slice(start, i));
      start = i + 1;
    }
  }
  parts.push(token.slice(start));
  return parts;
}

/**
 * Every token that might be a class name. Tailwind ignores the ones that are
 * not, so this over-collects rather than parse: each whitespace-separated token
 * is taken whole, split at quotes outside `[...]`, and split at every quote
 * and markup punctuation, which keeps arbitrary values (`data-[state=open]:x`,
 * `[&>svg]:size-4`, `className="after:content-['x']"`) intact in at least one cut.
 */
export function extractCandidates(content: string, out: Set<string>): void {
  for (const raw of content.split(/\s+/)) {
    if (!raw) continue;
    addCandidate(raw, out);
    for (const part of splitQuotesOutsideBrackets(raw)) addCandidate(part, out);
    for (const part of raw.split(/["'`]/)) {
      addCandidate(part, out);
      for (const sub of part.split(/[{}()<>;,]/)) addCandidate(sub, out);
    }
  }
}

/**
 * Compiles one stylesheet, or returns undefined when it is not a Tailwind
 * stylesheet. `path` is the stylesheet's own path and `base` its directory.
 */
export async function compileTailwind(
  css: string,
  path: string,
  base: string,
  host: TailwindHost,
): Promise<TailwindResult | undefined> {
  if (!usesTailwind(css)) return undefined;
  const compile = await host.loadCompile();
  const files = new Set<string>([path]);
  const compiler = await compile(css, {
    base,
    from: path,
    // Tailwind inlines every import into this stylesheet, which the bundler
    // then resolves from `base`: a relative url() has to be rebased to keep
    // pointing next to the sheet it was written in.
    loadStylesheet: async (id, from) => {
      const sheet = await host.loadStylesheet(id, from);
      files.add(sheet.path);
      if (sheet.base === base) return sheet;
      return {
        ...sheet,
        content: rewriteUrls(sheet.content, (url) => host.rebaseUrl(url, sheet.base, base)),
      };
    },
    loadModule: async (id, from, hint) => {
      const mod = await host.loadModule(id, from, hint);
      files.add(mod.path);
      return mod;
    },
  });
  // Features.None: a directive matched in a comment, say. The bundler's output
  // is what the author would get without Tailwind, so hand it back.
  if (compiler.features === 0) return undefined;

  const sources: TailwindSource[] = compiler.sources
    .filter((s) => !s.negated)
    .map((s) => ({ base: s.base, pattern: s.pattern }));
  let includeDefault = true;
  if (compiler.root === "none") {
    includeDefault = false;
  } else if (compiler.root) {
    includeDefault = false;
    sources.push(compiler.root);
  }
  const scanned = await host.scan(sources, includeDefault);
  const candidates = new Set<string>();
  for (const file of scanned.files) {
    files.add(file.path);
    extractCandidates(file.content, candidates);
  }
  return {
    css: compiler.build([...candidates]),
    files: [...files],
    dirs: scanned.dirs,
  };
}

const CSS_URL = /url\(\s*(["']?)([^"')]+?)\1\s*\)/g;
// Absolute, scheme-qualified (data:, https:, …), fragment or variable: not a file path.
const NOT_RELATIVE = /^(?:\/|#|[a-z][a-z0-9+.-]*:|var\()/i;

/** Applies `rebase` to every relative `url()` in `css`. */
export function rewriteUrls(css: string, rebase: (url: string) => string): string {
  return css.replace(CSS_URL, (match, quote, url) =>
    NOT_RELATIVE.test(url) ? match : `url(${quote}${rebase(url)}${quote})`,
  );
}

/** The part of a glob before its first wildcard segment: `src/**\/*.tsx` -> `src`. */
export function staticPrefix(pattern: string): string {
  const segments = pattern.split("/");
  const fixed: string[] = [];
  for (const segment of segments) {
    if (/[*?{}[\]!]/.test(segment)) break;
    fixed.push(segment);
  }
  return fixed.join("/");
}

/** Source file extensions scanned for class names by default. */
export const TAILWIND_SOURCE_EXTENSIONS = [
  ".tsx",
  ".ts",
  ".jsx",
  ".js",
  ".mjs",
  ".cjs",
  ".mts",
  ".cts",
  ".svelte",
  ".html",
  ".md",
  ".mdx",
];
