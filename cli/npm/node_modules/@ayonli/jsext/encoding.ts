/**
 * Utilities for encoding and decoding binary representations like hex and
 * base64 strings.
 * @module
 */

const encoder = new TextEncoder();

function asUint8Array(data: string | ArrayBufferLike | ArrayBufferView): Uint8Array {
    let bytes: Uint8Array;

    if (typeof data === "string") {
        bytes = encoder.encode(data);
    } else if (!(data instanceof Uint8Array)) {
        if (ArrayBuffer.isView(data)) {
            bytes = new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
        } else {
            bytes = new Uint8Array(data);
        }
    } else {
        bytes = data;
    }

    return bytes;
}

/**
 * Encodes the given data to a hex string.
 * 
 * @example
 * ```ts
 * import { encodeHex } from "@ayonli/jsext/encoding";
 * 
 * const hex = encodeHex("Hello, World!");
 * console.log(hex); // "48656c6c6f2c20576f726c6421"
 * 
 * const hex2 = encodeHex(new Uint8Array([72, 101, 108, 108, 111, 44, 32, 87, 111, 114, 108, 100, 33]));
 * console.log(hex2); // "48656c6c6f2c20576f726c6421"
 * ```
 */
export function encodeHex(data: string | ArrayBufferLike | ArrayBufferView): string {
    const bytes = asUint8Array(data);

    if (typeof Buffer === "function" && bytes instanceof Buffer) {
        return bytes.toString("hex");
    }

    return bytes.reduce((str, byte) => {
        return str + byte.toString(16).padStart(2, "0");
    }, "");
}

/**
 * Decodes the given hex string to a byte array.
 * 
 * @example
 * ```ts
 * import { decodeHex } from "@ayonli/jsext/encoding";
 * 
 * const data = decodeHex("48656c6c6f2c20576f726c6421");
 * console.log(data); // Uint8Array(13) [ 72, 101, 108, 108, 111, 44, 32, 87, 111, 114, 108, 100, 33 ]
 */
export function decodeHex(hex: string): Uint8Array {
    const bytes = new Uint8Array(hex.length / 2);

    for (let i = 0; i < hex.length; i += 2) {
        bytes[i / 2] = parseInt(hex.slice(i, i + 2), 16);
    }

    return bytes;
}

const base64Chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/**
 * Encodes the given data to a base64 string.
 * 
 * Unlike the built-in `btoa` function, this function is not limited to the
 * Latin1 range.
 * 
 * @example
 * ```ts
 * import { encodeBase64 } from "@ayonli/jsext/encoding";
 * 
 * const base64 = encodeBase64("Hello, World!");
 * console.log(base64); // "SGVsbG8sIFdvcmxkIQ=="
 * 
 * const base64_2 = encodeBase64("你好，世界！");
 * console.log(base64_2); // "5L2g5aW977yM5LiW55WM77yB"
 * ```
 */
export function encodeBase64(data: string | ArrayBufferLike | Uint8Array): string {
    const bytes = asUint8Array(data);

    if (typeof Buffer === "function" && bytes instanceof Buffer) {
        return bytes.toString("base64");
    }

    let result = "";
    let i: number;
    const l = bytes.length;

    for (i = 2; i < l; i += 3) {
        result += base64Chars[(bytes[i - 2]!) >> 2];
        result += base64Chars[
            (((bytes[i - 2]!) & 0x03) << 4) |
            ((bytes[i - 1]!) >> 4)
        ];
        result += base64Chars[
            (((bytes[i - 1]!) & 0x0f) << 2) |
            ((bytes[i]!) >> 6)
        ];
        result += base64Chars[(bytes[i]!) & 0x3f];
    }

    if (i === l + 1) {
        // 1 octet yet to write
        result += base64Chars[(bytes[i - 2]!) >> 2];
        result += base64Chars[((bytes[i - 2]!) & 0x03) << 4];
        result += "==";
    }

    if (i === l) {
        // 2 octets yet to write
        result += base64Chars[(bytes[i - 2]!) >> 2];
        result += base64Chars[
            (((bytes[i - 2]!) & 0x03) << 4) |
            ((bytes[i - 1]!) >> 4)
        ];
        result += base64Chars[((bytes[i - 1]!) & 0x0f) << 2];
        result += "=";
    }

    return result;
}

/**
 * Decodes the given base64 string to a byte array.
 * 
 * Unlike the built-in `atob` function, this function is not limited to the
 * Latin1 range.
 * 
 * @example
 * ```ts
 * import { decodeBase64 } from "@ayonli/jsext/encoding";
 * 
 * const data = decodeBase64("SGVsbG8sIFdvcmxkIQ==");
 * console.log(new TextDecoder.decode(data)); // "Hello, World!"
 * 
 * const data2 = decodeBase64("5L2g5aW977yM5LiW55WM77yB");
 * console.log(new TextDecoder.decode(data2)); // "你好，世界！"
 * ```
 */
export function decodeBase64(base64: string): Uint8Array {
    const padding = base64.endsWith("==") ? 2 : base64.endsWith("=") ? 1 : 0;
    const bytes = new Uint8Array((base64.length * 3 / 4) - padding);
    let i = 0;
    let j = 0;
    let c: number;
    let c1: number;
    let c2: number;
    let c3: number;

    while (i < base64.length) {
        c = base64Chars.indexOf(base64[i++]!);
        c1 = base64Chars.indexOf(base64[i++]!);
        c2 = base64Chars.indexOf(base64[i++]!);
        c3 = base64Chars.indexOf(base64[i++]!);

        bytes[j++] = (c << 2) | (c1 >> 4);
        if (i > base64.length + padding) break;
        bytes[j++] = ((c1 & 0xf) << 4) | (c2 >> 2);
        if (i > base64.length + padding) break;
        bytes[j++] = ((c2 & 0x3) << 6) | c3;
    }

    return bytes;
}
