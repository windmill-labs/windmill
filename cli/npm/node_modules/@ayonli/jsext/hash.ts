/**
 * Simplified hash functions for various data types, based on the Web Crypto API
 * and `crypto` package in Node.js.
 * @module
 */

import bytes from "./bytes.ts";
import { isDeno, isNodeLike } from "./env.ts";
import {
    type BufferSource,
    type DataSource,
    hash,
    hmac as _hmac,
    toBytes,
    sha1 as _sha1,
    sha256 as _sha256,
    sha512 as _sha512
} from "./hash/web.ts";

export type { BufferSource, DataSource };

export default hash;

async function nodeHash(
    algorithm: "sha1" | "sha256" | "sha512" | "md5",
    data: DataSource,
    encoding: "hex" | "base64" | undefined = undefined
): Promise<ArrayBuffer | string> {
    const crypto = await import("node:crypto");
    const bytes = await toBytes(data);
    const hash = crypto.createHash(algorithm);

    hash.update(bytes);

    if (encoding) {
        return hash.digest(encoding);
    } else {
        const result = hash.digest();
        // Truncate the buffer to the actual byte length so it's consistent with the web API.
        return result.buffer.slice(result.byteOffset, result.byteOffset + result.byteLength);
    }
}

/**
 * Calculates the SHA-1 hash of the given data.
 * 
 * @example
 * ```ts
 * import { sha1 } from "@ayonli/jsext/hash";
 * 
 * const buffer = await sha1("Hello, World!");
 * console.log(buffer); // ArrayBuffer(20) { ... }
 * ```
 */
export function sha1(data: DataSource): Promise<ArrayBuffer>;
/**
 * @example
 * ```ts
 * import { sha1 } from "@ayonli/jsext/hash";
 * 
 * const hex = await sha1("Hello, World!", "hex");
 * console.log(hex); // 0a0a9f2a6772942557ab5355d76af442f8f65e01
 * 
 * const base64 = await sha1("Hello, World!", "base64");
 * console.log(base64); // CgqfKmdylCVXq1NV12r0Qvj2XgE=
 * ```
 */
export function sha1(data: DataSource, encoding: "hex" | "base64"): Promise<string>;
export async function sha1(
    data: DataSource,
    encoding: "hex" | "base64" | undefined = undefined
): Promise<string | ArrayBuffer> {
    if (typeof crypto === "object") {
        return encoding ? _sha1(data, encoding) : _sha1(data);
    } else if (isDeno || isNodeLike) {
        return nodeHash("sha1", data, encoding);
    } else {
        throw new Error("Unsupported runtime");
    }
}

/**
 * Calculates the SHA-256 hash of the given data.
 * 
 * @example
 * ```ts
 * import { sha256 } from "@ayonli/jsext/hash";
 * 
 * const buffer = await sha256("Hello, World!");
 * console.log(buffer); // ArrayBuffer(32) { ... }
 * ```
 */
export async function sha256(data: DataSource): Promise<ArrayBuffer>;
/**
 * @example
 * ```ts
 * import { sha256 } from "@ayonli/jsext/hash";
 * 
 * const hex = await sha256("Hello, World!", "hex");
 * console.log(hex); // dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f
 * 
 * const base64 = await sha256("Hello, World!", "base64");
 * console.log(base64); // 3/1gIbsr1bCvZ2KQgJ7DpTGR3YHH9wpLKGiKNiGCmG8=
 * ```
 */
export async function sha256(data: DataSource, encoding: "hex" | "base64"): Promise<string>;
export async function sha256(
    data: DataSource,
    encoding: "hex" | "base64" | undefined = undefined
): Promise<string | ArrayBuffer> {
    if (typeof crypto === "object") {
        return encoding ? _sha256(data, encoding) : _sha256(data);
    } else if (isDeno || isNodeLike) {
        return nodeHash("sha256", data, encoding);
    } else {
        throw new Error("Unsupported runtime");
    }
}

/**
 * Calculates the SHA-512 hash of the given data.
 * 
 * @example
 * ```ts
 * import { sha512 } from "@ayonli/jsext/hash";
 * 
 * const buffer = await sha512("Hello, World!");
 * console.log(buffer); // ArrayBuffer(64) { ... }
 * ```
 */
export async function sha512(data: DataSource): Promise<ArrayBuffer>;
/**
 * @example
 * ```ts
 * import { sha512 } from "@ayonli/jsext/hash";
 * 
 * const hex = await sha512("Hello, World!", "hex");
 * console.log(hex);
 * // 374d794a95cdcfd8b35993185fef9ba368f160d8daf432d08ba9f1ed1e5abe6cc69291e0fa2fe0006a52570ef18c19def4e617c33ce52ef0a6e5fbe318cb0387
 * 
 * const base64 = await sha512("Hello, World!", "base64");
 * console.log(base64);
 * // N015SpXNz9izWZMYX++bo2jxYNja9DLQi6nx7R5avmzGkpHg+i/gAGpSVw7xjBne9OYXwzzlLvCm5fvjGMsDhw==
 * ``` 
 */
export async function sha512(data: DataSource, encoding: "hex" | "base64"): Promise<string>;
export async function sha512(
    data: DataSource,
    encoding: "hex" | "base64" | undefined = undefined
): Promise<string | ArrayBuffer> {
    if (typeof crypto === "object") {
        return encoding ? _sha512(data, encoding) : _sha512(data);
    } else if (isDeno || isNodeLike) {
        return nodeHash("sha512", data, encoding);
    } else {
        throw new Error("Unsupported runtime");
    }
}

/**
 * Calculates the MD5 hash of the given data.
 * 
 * NOTE: This function is not available in the browser.
 * 
 * @example
 * ```ts
 * import { md5 } from "@ayonli/jsext/hash";
 * 
 * const buffer = await md5("Hello, World!");
 * console.log(buffer); // ArrayBuffer(16) { ... }
 * ```
 */
export async function md5(data: DataSource): Promise<ArrayBuffer>;
/**
 * @example
 * ```ts
 * import { md5 } from "@ayonli/jsext/hash";
 * 
 * const hex = await md5("Hello, World!", "hex");
 * console.log(hex); // 65a8e27d8879283831b664bd8b7f0ad4
 * 
 * const base64 = await md5("Hello, World!", "base64");
 * console.log(base64); // ZajifYh5KDgxtmS9i38K1A==
 * ```
 */
export async function md5(data: DataSource, encoding: "hex" | "base64"): Promise<string>;
export async function md5(
    data: DataSource,
    encoding: "hex" | "base64" | undefined = undefined
): Promise<string | ArrayBuffer> {
    if (isDeno || isNodeLike) {
        return nodeHash("md5", data, encoding);
    } else {
        throw new Error("Unsupported runtime");
    }
}

export async function hmac(
    algorithm: "sha1" | "sha256" | "sha512",
    key: BufferSource,
    data: DataSource
): Promise<ArrayBuffer>;
export async function hmac(
    algorithm: "sha1" | "sha256" | "sha512",
    key: BufferSource,
    data: DataSource,
    encoding: "hex" | "base64"
): Promise<string>;
export async function hmac(
    algorithm: "sha1" | "sha256" | "sha512",
    key: BufferSource,
    data: DataSource,
    encoding: "hex" | "base64" | undefined = undefined
): Promise<string | ArrayBuffer> {
    if (typeof crypto === "object") {
        return encoding ? _hmac(algorithm, key, data, encoding) : _hmac(algorithm, key, data);
    } else if (isDeno || isNodeLike) {
        const crypto = await import("node:crypto");
        const binary = await toBytes(data);
        const hash = crypto.createHmac(algorithm, bytes(key));

        hash.update(binary);

        if (encoding) {
            return hash.digest(encoding);
        } else {
            const result = hash.digest();
            // Truncate the buffer to the actual byte length so it's consistent with the web API.
            return result.buffer.slice(result.byteOffset, result.byteOffset + result.byteLength);
        }
    } else {
        throw new Error("Unsupported runtime");
    }
}
