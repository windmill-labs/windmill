import {
  KeyMap,
  KeyMapCtrl,
  KeyMapShift,
  SpecialKeyMap,
} from "./_key_codes.js";

// https://en.wikipedia.org/wiki/ANSI_escape_code
// https://github.com/nodejs/node/blob/v13.13.0/lib/internal/readline/utils.js

const kUTF16SurrogateThreshold = 0x10000; // 2 ** 16
const kEscape = "\x1b";

/** KeyCode object. */
export interface KeyCode {
  /** Key name. */
  name?: string;
  /** Key sequence. */
  sequence?: string;
  /** Key code. */
  code?: string;
  /** Indicates if the ctrl key is pressed. */
  ctrl?: boolean;
  /** Indicates if the meta key is pressed. */
  meta?: boolean;
  /** Indicates if the shift key is pressed. */
  shift?: boolean;
  /** Key string value. */
  char?: string;
}

/**
 * Parse ansi escape sequence.
 *
 * @param data Ansi escape sequence.
 *
 * ```ts
 * import { parse } from "./mod.ts";
 *
 * parse("\x04\x18");
 * ```
 *
 * ```json
 * [
 *   KeyCode { name: "d", sequence: "\x04", ctrl: true, meta: false, shift: false },
 *   KeyCode { name: "x", sequence: "\x18", ctrl: true, meta: false, shift: false },
 * ]
 * ```
 */
export function parse(data: Uint8Array | string): KeyCode[] {
  /*
   * Some patterns seen in terminal key escape codes, derived from combos seen
   * at http://www.midnight-commander.org/browser/lib/tty/key.c
   *
   * ESC letter
   * ESC [ letter
   * ESC [ modifier letter
   * ESC [ 1 ; modifier letter
   * ESC [ num char
   * ESC [ num ; modifier char
   * ESC O letter
   * ESC O modifier letter
   * ESC O 1 ; modifier letter
   * ESC N letter
   * ESC [ [ num ; modifier char
   * ESC [ [ 1 ; modifier letter
   * ESC ESC [ num char
   * ESC ESC O letter
   *
   * - char is usually ~ but $ and ^ also happen with rxvt
   * - modifier is 1 +
   *               (shift     * 1) +
   *               (left_alt  * 2) +
   *               (ctrl      * 4) +
   *               (right_alt * 8)
   * - two leading ESCs apparently mean the same as one leading ESC
   */
  let index = -1;
  const keys: KeyCode[] = [];
  const input: string = data instanceof Uint8Array
    ? new TextDecoder().decode(data)
    : data;

  const hasNext = () => input.length - 1 >= index + 1;
  const next = () => input[++index];

  parseNext();

  return keys;

  function parseNext() {
    let ch: string = next();
    let s: string = ch;
    let escaped = false;

    const key: KeyCode = {
      name: undefined,
      char: undefined,
      sequence: undefined,
      code: undefined,
      ctrl: false,
      meta: false,
      shift: false,
    };

    if (ch === kEscape && hasNext()) {
      escaped = true;
      s += ch = next();

      if (ch === kEscape) {
        s += ch = next();
      }
    }

    if (escaped && (ch === "O" || ch === "[")) {
      // ANSI escape sequence
      let code: string = ch;
      let modifier = 0;

      if (ch === "O") {
        // ESC O letter
        // ESC O modifier letter
        s += ch = next();

        if (ch >= "0" && ch <= "9") {
          modifier = (Number(ch) >> 0) - 1;
          s += ch = next();
        }

        code += ch;
      } else if (ch === "[") {
        // ESC [ letter
        // ESC [ modifier letter
        // ESC [ [ modifier letter
        // ESC [ [ num char
        s += ch = next();

        if (ch === "[") {
          // \x1b[[A
          //      ^--- escape codes might have a second bracket
          code += ch;
          s += ch = next();
        }

        /*
         * Here and later we try to buffer just enough data to get
         * a complete ascii sequence.
         *
         * We have basically two classes of ascii characters to process:
         *
         * 1. `\x1b[24;5~` should be parsed as { code: '[24~', modifier: 5 }
         *
         * This particular example is featuring Ctrl+F12 in xterm.
         *
         *  - `;5` part is optional, e.g. it could be `\x1b[24~`
         *  - first part can contain one or two digits
         *
         * So the generic regexp is like /^\d\d?(;\d)?[~^$]$/
         *
         * 2. `\x1b[1;5H` should be parsed as { code: '[H', modifier: 5 }
         *
         * This particular example is featuring Ctrl+Home in xterm.
         *
         *  - `1;5` part is optional, e.g. it could be `\x1b[H`
         *  - `1;` part is optional, e.g. it could be `\x1b[5H`
         *
         * So the generic regexp is like /^((\d;)?\d)?[A-Za-z]$/
         */
        const cmdStart: number = s.length - 1;

        // Skip one or two leading digits
        if (ch >= "0" && ch <= "9") {
          s += ch = next();

          if (ch >= "0" && ch <= "9") {
            s += ch = next();
          }
        }

        // skip modifier
        if (ch === ";") {
          s += ch = next();

          if (ch >= "0" && ch <= "9") {
            s += next();
          }
        }

        /*
         * We buffered enough data, now trying to extract code
         * and modifier from it
         */
        const cmd: string = s.slice(cmdStart);
        let match: RegExpMatchArray | null;

        if ((match = cmd.match(/^(\d\d?)(;(\d))?([~^$])$/))) {
          code += match[1] + match[4];
          modifier = (Number(match[3]) || 1) - 1;
        } else if ((match = cmd.match(/^((\d;)?(\d))?([A-Za-z])$/))) {
          code += match[4];
          modifier = (Number(match[3]) || 1) - 1;
        } else {
          code += cmd;
        }
      }

      // Parse the key modifier
      key.ctrl = !!(modifier & 4);
      key.meta = !!(modifier & 10);
      key.shift = !!(modifier & 1);
      key.code = code;

      // Parse the key itself
      if (code in KeyMap) {
        key.name = KeyMap[code];
      } else if (code in KeyMapShift) {
        key.name = KeyMapShift[code];
        key.shift = true;
      } else if (code in KeyMapCtrl) {
        key.name = KeyMapCtrl[code];
        key.ctrl = true;
      } else {
        key.name = "undefined";
      }
    } else if (ch in SpecialKeyMap) {
      key.name = SpecialKeyMap[ch];
      key.meta = escaped;

      if (key.name === "space") {
        key.char = ch;
      }
    } else if (!escaped && ch <= "\x1a") {
      // ctrl+letter
      key.name = String.fromCharCode(
        ch.charCodeAt(0) + "a".charCodeAt(0) - 1,
      );
      key.ctrl = true;

      key.char = key.name;
    } else if (/^[0-9A-Za-z]$/.test(ch)) {
      // Letter, number, shift+letter
      key.name = ch.toLowerCase();
      key.shift = /^[A-Z]$/.test(ch);
      key.meta = escaped;
      key.char = ch;
    } else if (escaped) {
      // Escape sequence timeout
      key.name = ch.length ? undefined : "escape";
      key.meta = true;
    } else {
      key.name = ch;
      key.char = ch;
    }

    key.sequence = s;

    if (
      (s.length !== 0 && (key.name !== undefined || escaped)) ||
      charLengthAt(s, 0) === s.length
    ) {
      keys.push(key);
    } else {
      throw new Error("Unrecognized or broken escape sequence");
    }

    if (hasNext()) {
      parseNext();
    }
  }
}

function charLengthAt(str: string, i: number): number {
  const pos: number | undefined = str.codePointAt(i);
  if (typeof pos === "undefined") {
    // Pretend to move to the right. This is necessary to autocomplete while
    // moving to the right.
    return 1;
  }
  return pos >= kUTF16SurrogateThreshold ? 2 : 1;
}
