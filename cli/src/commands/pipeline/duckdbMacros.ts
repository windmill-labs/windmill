// Faithful TS port of the lexical rules in
// `backend/parsers/windmill-parser/src/duckdb_macros.rs` — keep the two in
// lockstep. Deliberately lexical, not an AST parse: over-matching a call only
// draws a spurious edge, never a wrong run. A macro reached only through dynamic
// SQL (`query('…')`) is invisible to call detection here as on the server —
// `// use <lib>` forces the whole-library edge instead.

export type ParsedMacro = {
  // Lowercased bare identifier (DuckDB identifiers are case-insensitive unquoted;
  // qualified / quoted names are rejected, matching the backend).
  name: string;
  // Verbatim text inside the parameter parens (may be empty).
  params: string;
  isTable: boolean;
};

// Case-insensitive whole-word prefix strip (whitespace-bounded). Returns the
// remainder with leading whitespace trimmed, or null when `s` does not start with
// `kw` as a whole word. `kw` must be lowercase. Mirrors `strip_kw`.
function stripKw(s: string, kw: string): string | null {
  if (s.length < kw.length) return null;
  if (s.slice(0, kw.length).toLowerCase() !== kw) return null;
  const after = s.slice(kw.length);
  if (after.length === 0 || /^\s/.test(after)) return after.replace(/^\s+/, "");
  return null;
}

function isIdent(s: string): boolean {
  return /^[A-Za-z_][A-Za-z0-9_]*$/.test(s);
}

// Split a script body into `;`-terminated statements with line comments
// (`--`, `//`), block comments (`/* */`), and quoted spans stripped/preserved as
// in the SQL executor. Char-wise (not byte-wise) so multi-byte text survives.
// Mirrors `split_statements`.
export function splitStatements(sql: string): string[] {
  const out: string[] = [];
  let cur = "";
  const chars = [...sql];
  let i = 0;
  const n = chars.length;
  while (i < n) {
    const c = chars[i];
    // line comment: `--` (SQL) or `//` (Windmill annotation prefix)
    if (
      (c === "-" && i + 1 < n && chars[i + 1] === "-") ||
      (c === "/" && i + 1 < n && chars[i + 1] === "/")
    ) {
      while (i < n && chars[i] !== "\n") i += 1;
      continue;
    }
    // block comment
    if (c === "/" && i + 1 < n && chars[i + 1] === "*") {
      i += 2;
      while (i + 1 < n && !(chars[i] === "*" && chars[i + 1] === "/")) i += 1;
      i += 2;
      continue;
    }
    // single-quoted string ('' escapes an embedded quote)
    if (c === "'") {
      cur += c;
      i += 1;
      while (i < n) {
        cur += chars[i];
        if (chars[i] === "'") {
          if (i + 1 < n && chars[i + 1] === "'") {
            cur += "'";
            i += 2;
            continue;
          }
          i += 1;
          break;
        }
        i += 1;
      }
      continue;
    }
    // double-quoted identifier
    if (c === '"') {
      cur += c;
      i += 1;
      while (i < n) {
        cur += chars[i];
        if (chars[i] === '"') {
          i += 1;
          break;
        }
        i += 1;
      }
      continue;
    }
    if (c === ";") {
      const t = cur.trim();
      if (t !== "") out.push(t);
      cur = "";
      i += 1;
      continue;
    }
    cur += c;
    i += 1;
  }
  const t = cur.trim();
  if (t !== "") out.push(t);
  return out;
}

// Parse one comment-free, `;`-less statement as a CREATE MACRO. Returns null when
// the statement is not a well-formed macro definition (unlike the backend, which
// distinguishes "not macro-shaped" from "malformed" to surface deploy errors —
// the local graph is lenient and simply skips anything it can't read). Mirrors
// `parse_create_macro` for the shape it accepts.
export function parseCreateMacro(stmt: string): ParsedMacro | null {
  let rest = stripKw(stmt.trim(), "create");
  if (rest === null) return null;
  const afterOr = stripKw(rest, "or");
  if (afterOr !== null) {
    const afterReplace = stripKw(afterOr, "replace");
    if (afterReplace === null) return null; // CREATE OR <not replace>
    rest = afterReplace;
  }
  const afterTemp = stripKw(rest, "temp") ?? stripKw(rest, "temporary");
  if (afterTemp !== null) rest = afterTemp;
  // `FUNCTION` is DuckDB's alias for `MACRO`.
  const afterMacro = stripKw(rest, "macro") ?? stripKw(rest, "function");
  if (afterMacro === null) return null;
  rest = afterMacro;

  const nameEndMatch = rest.match(/[\s(]/);
  const nameEnd = nameEndMatch ? nameEndMatch.index! : rest.length;
  const rawName = rest.slice(0, nameEnd);
  if (rawName === "" || rawName.includes(".") || rawName.includes('"') || !isIdent(rawName)) {
    return null;
  }
  const name = rawName.toLowerCase();

  rest = rest.slice(nameEnd).replace(/^\s+/, "");
  if (!rest.startsWith("(")) return null;
  // Balanced-paren scan for the verbatim param list; skip quoted spans (a default
  // value may contain a paren).
  const pchars = [...rest];
  let depth = 0;
  let j = 0;
  let close = -1;
  while (j < pchars.length) {
    const ch = pchars[j];
    if (ch === "(") depth += 1;
    else if (ch === ")") {
      depth -= 1;
      if (depth === 0) {
        close = j;
        break;
      }
    } else if (ch === "'" || ch === '"') {
      const q = ch;
      j += 1;
      while (j < pchars.length && pchars[j] !== q) j += 1;
    }
    j += 1;
  }
  if (close === -1) return null; // unbalanced
  const params = pchars.slice(1, close).join("").trim();

  const afterParams = pchars.slice(close + 1).join("").replace(/^\s+/, "");
  let body = stripKw(afterParams, "as");
  if (body === null) return null;
  let isTable = false;
  const afterTable = stripKw(body, "table");
  if (afterTable !== null) {
    body = afterTable;
    isTable = true;
  }
  const trimmedBody = body.replace(/;+\s*$/, "").trim();
  if (trimmedBody === "") return null; // empty body
  return { name, params, isTable };
}

// All well-formed macro definitions in a `// macros` library body, in source
// order. Non-macro statements (setup: ATTACH/INSTALL/… ) are skipped.
export function parseMacroLibrary(sql: string): ParsedMacro[] {
  const out: ParsedMacro[] = [];
  for (const stmt of splitStatements(sql)) {
    const m = parseCreateMacro(stmt);
    if (m) out.push(m);
  }
  return out;
}

// Names from `names` that `sql` calls: an identifier token immediately followed
// by `(` (after optional whitespace), not `.`-qualified, outside strings and
// comments. Lexical (over-matching only adds an unused edge). Mirrors
// `detect_macro_calls`.
export function detectMacroCalls(sql: string, names: Set<string>): Set<string> {
  const found = new Set<string>();
  if (names.size === 0) return found;
  for (const stmt of splitStatements(sql)) {
    const chars = [...stmt];
    const n = chars.length;
    let i = 0;
    let prev: string | undefined = undefined;
    while (i < n) {
      const c = chars[i];
      if (c === "'" || c === '"') {
        const q = c;
        i += 1;
        while (i < n && chars[i] !== q) i += 1;
        i += 1;
        prev = q;
        continue;
      }
      const isIdentStart = /[A-Za-z_]/.test(c);
      const prevBlocks = prev !== undefined && /[.A-Za-z0-9_]/.test(prev);
      if (isIdentStart && !prevBlocks) {
        const start = i;
        while (i < n && /[A-Za-z0-9_]/.test(chars[i])) i += 1;
        const word = chars.slice(start, i).join("").toLowerCase();
        let k = i;
        while (k < n && /\s/.test(chars[k])) k += 1;
        if (k < n && chars[k] === "(" && names.has(word)) found.add(word);
        prev = chars[i - 1];
        continue;
      }
      prev = c;
      i += 1;
    }
  }
  return found;
}

// Scan the LEADING comment header for the `// macros` marker and `// use <lib>`
// annotations, independent of the wasm parser (which drops both). Faithful mirror
// of the canonical `parse_pipeline_annotations` (asset_parser.rs): each line's
// prefix is stripped as `//`, `--`, or `#` REGARDLESS of language (a DuckDB
// library may legitimately head its annotations with `// macros`), scanning stops
// at the first non-comment line, the marker must stand alone on its line, and a
// `// use` target is a single whitespace-free token containing `/`.
export function parseMacroAnnotations(
  content: string,
): { macros: boolean; useLibs: string[] } {
  let macros = false;
  const useLibs: string[] = [];
  for (const raw of content.split("\n")) {
    const line = raw.replace(/^\s+/, "");
    if (line === "") continue;
    let rest: string;
    if (line.startsWith("//") || line.startsWith("--")) rest = line.slice(2);
    else if (line.startsWith("#")) rest = line.slice(1);
    else break; // first line of actual code ends the annotation header
    rest = rest.replace(/^\s+/, "");
    // marker: `macros` as the only content on the line (rejects `macros_v2`,
    // `macros are below`)
    if (/^macros\s*$/.test(rest)) {
      macros = true;
      continue;
    }
    // `use <lib>`: a single whitespace-free path token (must contain `/`, so
    // prose like `use this to …` is dropped)
    const u = rest.match(/^use\s+(\S+)\s*$/);
    if (u && u[1].includes("/") && !useLibs.includes(u[1])) useLibs.push(u[1]);
  }
  return { macros, useLibs };
}
