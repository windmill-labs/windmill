// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.

/**
 * Options for {@linkcode globToRegExp}, {@linkcode joinGlobs},
 * {@linkcode normalizeGlob} and {@linkcode expandGlob}.
 */
export interface GlobOptions {
  /** Extended glob syntax.
   * See https://www.linuxjournal.com/content/bash-extended-globbing.
   *
   * @default {true}
   */
  extended?: boolean;
  /** Globstar syntax.
   * See https://www.linuxjournal.com/content/globstar-new-bash-globbing-option.
   * If false, `**` is treated like `*`.
   *
   * @default {true}
   */
  globstar?: boolean;
  /**
   * Whether globstar should be case-insensitive.
   *
   * @default {false}
   */
  caseInsensitive?: boolean;
}

const REG_EXP_ESCAPE_CHARS = [
  "!",
  "$",
  "(",
  ")",
  "*",
  "+",
  ".",
  "=",
  "?",
  "[",
  "\\",
  "^",
  "{",
  "|",
] as const;
const RANGE_ESCAPE_CHARS = ["-", "\\", "]"] as const;

type RegExpEscapeChar = typeof REG_EXP_ESCAPE_CHARS[number];
type RangeEscapeChar = typeof RANGE_ESCAPE_CHARS[number];
type EscapeChar = RegExpEscapeChar | RangeEscapeChar;

export interface GlobConstants {
  sep: string;
  sepMaybe: string;
  seps: string[];
  globstar: string;
  wildcard: string;
  escapePrefix: string;
}

export function _globToRegExp(
  c: GlobConstants,
  glob: string,
  {
    extended = true,
    globstar: globstarOption = true,
    // os = osType,
    caseInsensitive = false,
  }: GlobOptions = {},
): RegExp {
  if (glob === "") {
    return /(?!)/;
  }

  // Remove trailing separators.
  let newLength = glob.length;
  for (; newLength > 1 && c.seps.includes(glob[newLength - 1]!); newLength--);
  glob = glob.slice(0, newLength);

  let regExpString = "";

  // Terminates correctly. Trust that `j` is incremented every iteration.
  for (let j = 0; j < glob.length;) {
    let segment = "";
    const groupStack: string[] = [];
    let inRange = false;
    let inEscape = false;
    let endsWithSep = false;
    let i = j;

    // Terminates with `i` at the non-inclusive end of the current segment.
    for (; i < glob.length && !c.seps.includes(glob[i]!); i++) {
      if (inEscape) {
        inEscape = false;
        const escapeChars = (inRange
          ? RANGE_ESCAPE_CHARS
          : REG_EXP_ESCAPE_CHARS) as unknown as EscapeChar[];
        segment += escapeChars.includes(glob[i]! as EscapeChar)
          ? `\\${glob[i]}`
          : glob[i];
        continue;
      }

      if (glob[i] === c.escapePrefix) {
        inEscape = true;
        continue;
      }

      if (glob[i] === "[") {
        if (!inRange) {
          inRange = true;
          segment += "[";
          if (glob[i + 1] === "!") {
            i++;
            segment += "^";
          } else if (glob[i + 1] === "^") {
            i++;
            segment += "\\^";
          }
          continue;
        } else if (glob[i + 1] === ":") {
          let k = i + 1;
          let value = "";
          while (glob[k + 1] !== undefined && glob[k + 1] !== ":") {
            value += glob[k + 1];
            k++;
          }
          if (glob[k + 1] === ":" && glob[k + 2] === "]") {
            i = k + 2;
            if (value === "alnum") segment += "\\dA-Za-z";
            else if (value === "alpha") segment += "A-Za-z";
            else if (value === "ascii") segment += "\x00-\x7F";
            else if (value === "blank") segment += "\t ";
            else if (value === "cntrl") segment += "\x00-\x1F\x7F";
            else if (value === "digit") segment += "\\d";
            else if (value === "graph") segment += "\x21-\x7E";
            else if (value === "lower") segment += "a-z";
            else if (value === "print") segment += "\x20-\x7E";
            else if (value === "punct") {
              segment += "!\"#$%&'()*+,\\-./:;<=>?@[\\\\\\]^_‘{|}~";
            } else if (value === "space") segment += "\\s\v";
            else if (value === "upper") segment += "A-Z";
            else if (value === "word") segment += "\\w";
            else if (value === "xdigit") segment += "\\dA-Fa-f";
            continue;
          }
        }
      }

      if (glob[i] === "]" && inRange) {
        inRange = false;
        segment += "]";
        continue;
      }

      if (inRange) {
        segment += glob[i];
        continue;
      }

      if (
        glob[i] === ")" && groupStack.length > 0 &&
        groupStack[groupStack.length - 1] !== "BRACE"
      ) {
        segment += ")";
        const type = groupStack.pop()!;
        if (type === "!") {
          segment += c.wildcard;
        } else if (type !== "@") {
          segment += type;
        }
        continue;
      }

      if (
        glob[i] === "|" && groupStack.length > 0 &&
        groupStack[groupStack.length - 1] !== "BRACE"
      ) {
        segment += "|";
        continue;
      }

      if (glob[i] === "+" && extended && glob[i + 1] === "(") {
        i++;
        groupStack.push("+");
        segment += "(?:";
        continue;
      }

      if (glob[i] === "@" && extended && glob[i + 1] === "(") {
        i++;
        groupStack.push("@");
        segment += "(?:";
        continue;
      }

      if (glob[i] === "?") {
        if (extended && glob[i + 1] === "(") {
          i++;
          groupStack.push("?");
          segment += "(?:";
        } else {
          segment += ".";
        }
        continue;
      }

      if (glob[i] === "!" && extended && glob[i + 1] === "(") {
        i++;
        groupStack.push("!");
        segment += "(?!";
        continue;
      }

      if (glob[i] === "{") {
        groupStack.push("BRACE");
        segment += "(?:";
        continue;
      }

      if (glob[i] === "}" && groupStack[groupStack.length - 1] === "BRACE") {
        groupStack.pop();
        segment += ")";
        continue;
      }

      if (glob[i] === "," && groupStack[groupStack.length - 1] === "BRACE") {
        segment += "|";
        continue;
      }

      if (glob[i] === "*") {
        if (extended && glob[i + 1] === "(") {
          i++;
          groupStack.push("*");
          segment += "(?:";
        } else {
          const prevChar = glob[i - 1];
          let numStars = 1;
          while (glob[i + 1] === "*") {
            i++;
            numStars++;
          }
          const nextChar = glob[i + 1];
          if (
            globstarOption && numStars === 2 &&
            [...c.seps, undefined].includes(prevChar) &&
            [...c.seps, undefined].includes(nextChar)
          ) {
            segment += c.globstar;
            endsWithSep = true;
          } else {
            segment += c.wildcard;
          }
        }
        continue;
      }

      segment += REG_EXP_ESCAPE_CHARS.includes(glob[i]! as RegExpEscapeChar)
        ? `\\${glob[i]}`
        : glob[i];
    }

    // Check for unclosed groups or a dangling backslash.
    if (groupStack.length > 0 || inRange || inEscape) {
      // Parse failure. Take all characters from this segment literally.
      segment = "";
      for (const c of glob.slice(j, i)) {
        segment += REG_EXP_ESCAPE_CHARS.includes(c as RegExpEscapeChar)
          ? `\\${c}`
          : c;
        endsWithSep = false;
      }
    }

    regExpString += segment;
    if (!endsWithSep) {
      regExpString += i < glob.length ? c.sep : c.sepMaybe;
      endsWithSep = true;
    }

    // Terminates with `i` at the start of the next segment.
    while (c.seps.includes(glob[i]!)) i++;

    j = i;
  }

  regExpString = `^${regExpString}$`;
  return new RegExp(regExpString, caseInsensitive ? "i" : "");
}
