// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.

/**
 * Utilities for working with {@link https://en.wikipedia.org/wiki/Ascii85 | ascii85} encoding.
 *
 * ## Specifying a standard and delimiter
 *
 * By default, all functions are using the most popular Adobe version of ascii85
 * and not adding any delimiter. However, there are three more standards
 * supported - btoa (different delimiter and additional compression of 4 bytes
 * equal to 32), {@link https://rfc.zeromq.org/spec/32/ | Z85} and
 * {@link https://www.rfc-editor.org/rfc/rfc1924.html | RFC 1924}. It's possible to use a
 * different encoding by specifying it in `options` object as a second parameter.
 *
 * Similarly, it's possible to make `encode` add a delimiter (`<~` and `~>` for
 * Adobe, `xbtoa Begin` and `xbtoa End` with newlines between the delimiters and
 * encoded data for btoa. Checksums for btoa are not supported. Delimiters are not
 * supported by other encodings.)
 *
 * @module
 */

import { validateBinaryLike } from "./_validate_binary_like.js";

/**
 * Supported ascii85 standards for {@linkcode EncodeAscii85Options} and
 * {@linkcode DecodeAscii85Options}.
 */
export type Ascii85Standard = "Adobe" | "btoa" | "RFC 1924" | "Z85";

/** Options for {@linkcode encodeAscii85}. */
export interface EncodeAscii85Options {
  /**
   * Character set and delimiter (if supported and used).
   *
   * @default {"Adobe"}
   */
  standard?: Ascii85Standard;
  /**
   * Whether to use a delimiter (if supported).
   *
   * @default {false}
   */
  delimiter?: boolean;
}

const rfc1924 =
  "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz!#$%&()*+-;<=>?@^_`{|}~" as const;
const Z85 =
  "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ.-:+=^!/*?&<>()[]{}@%$#" as const;

/**
 * Converts data into an ascii85-encoded string.
 *
 * @param data The data to encode.
 * @param options Options for encoding.
 *
 * @returns The ascii85-encoded string.
 *
 * @example Usage
 * ```ts
 * import { encodeAscii85 } from "@std/encoding/ascii85";
 * import { assertEquals } from "@std/assert";
 *
 * assertEquals(encodeAscii85("Hello world!"), "87cURD]j7BEbo80");
 * ```
 */
export function encodeAscii85(
  data: ArrayBuffer | Uint8Array | string,
  options: EncodeAscii85Options = {},
): string {
  let uint8 = validateBinaryLike(data);

  const { standard = "Adobe" } = options;

  let output: string[] = [];
  let v: number;
  let n = 0;
  let difference = 0;
  if (uint8.length % 4 !== 0) {
    const tmp = uint8;
    difference = 4 - (tmp.length % 4);
    uint8 = new Uint8Array(tmp.length + difference);
    uint8.set(tmp);
  }
  const view = new DataView(uint8.buffer, uint8.byteOffset, uint8.byteLength);
  for (let i = 0; i < uint8.length; i += 4) {
    v = view.getUint32(i);
    // Adobe and btoa standards compress 4 zeroes to single "z" character
    if (
      (standard === "Adobe" || standard === "btoa") &&
      v === 0 &&
      i < uint8.length - difference - 3
    ) {
      output[n++] = "z";
      continue;
    }
    // btoa compresses 4 spaces - that is, bytes equal to 32 - into single "y" character
    if (standard === "btoa" && v === 538976288) {
      output[n++] = "y";
      continue;
    }
    for (let j = 4; j >= 0; j--) {
      output[n + j] = String.fromCharCode((v % 85) + 33);
      v = Math.trunc(v / 85);
    }
    n += 5;
  }
  switch (standard) {
    case "Adobe":
      if (options?.delimiter) {
        return `<~${output.slice(0, output.length - difference).join("")}~>`;
      }
      break;
    case "btoa":
      if (options?.delimiter) {
        return `xbtoa Begin\n${
          output
            .slice(0, output.length - difference)
            .join("")
        }\nxbtoa End`;
      }
      break;
    case "RFC 1924":
      output = output.map((val) => rfc1924[val.charCodeAt(0) - 33]!);
      break;
    case "Z85":
      output = output.map((val) => Z85[val.charCodeAt(0) - 33]!);
      break;
  }
  return output.slice(0, output.length - difference).join("");
}

/** Options for {@linkcode decodeAscii85}. */
export type DecodeAscii85Options = Omit<EncodeAscii85Options, "delimiter">;

/**
 * Decodes a ascii85-encoded string.
 *
 * @param ascii85 The ascii85-encoded string to decode.
 * @param options Options for decoding.
 * @returns The decoded data.
 *
 * @example Usage
 * ```ts
 * import { decodeAscii85 } from "@std/encoding/ascii85";
 * import { assertEquals } from "@std/assert";
 *
 * assertEquals(
 *   decodeAscii85("87cURD]j7BEbo80"),
 *   new TextEncoder().encode("Hello world!"),
 * );
 * ```
 */
export function decodeAscii85(
  ascii85: string,
  options: DecodeAscii85Options = {},
): Uint8Array {
  const { standard = "Adobe" } = options;

  // translate all encodings to most basic adobe/btoa one and decompress some special characters ("z" and "y")
  switch (standard) {
    case "Adobe":
      ascii85 = ascii85.replaceAll(/(<~|~>)/g, "").replaceAll("z", "!!!!!");
      break;
    case "btoa":
      ascii85 = ascii85
        .replaceAll(/(xbtoa Begin|xbtoa End|\n)/g, "")
        .replaceAll("z", "!!!!!")
        .replaceAll("y", "+<VdL");
      break;
    case "RFC 1924":
      ascii85 = ascii85.replaceAll(
        /./g,
        (match) => String.fromCharCode(rfc1924.indexOf(match) + 33),
      );
      break;
    case "Z85":
      ascii85 = ascii85.replaceAll(
        /./g,
        (match) => String.fromCharCode(Z85.indexOf(match) + 33),
      );
      break;
  }
  // remove all invalid characters
  ascii85 = ascii85.replaceAll(/[^!-u]/g, "");
  const len = ascii85.length;
  const output = new Uint8Array(len + 4 - (len % 4));
  const view = new DataView(output.buffer);
  let v = 0;
  let n = 0;
  let max = 0;
  for (let i = 0; i < len;) {
    for (max += 5; i < max; i++) {
      v = v * 85 + (i < len ? ascii85.charCodeAt(i) : 117) - 33;
    }
    view.setUint32(n, v);
    v = 0;
    n += 4;
  }
  return output.slice(0, Math.trunc(len * 0.8));
}
