import bytes, { text } from "../bytes.ts";
import { readAsArrayBuffer } from "../reader.ts";

export type BufferSource = string | ArrayBuffer | ArrayBufferView;

export type DataSource = BufferSource | ReadableStream<Uint8Array> | Blob;

function strHash(str: string): number {
    let hash = 5381;
    let i = str.length;

    while (i) {
        hash = (hash * 33) ^ str.charCodeAt(--i);
    }

    /* JavaScript does bitwise operations (like XOR, above) on 32-bit signed
     * integers. Since we want the results to be always positive, convert the
     * signed int to an unsigned by doing an unsigned bitshift. */
    return hash >>> 0;
}

/**
 * Calculates the hash of the given data.
 * 
 * This function uses the same algorithm as the [string-hash](https://www.npmjs.com/package/string-hash)
 * package, non-string data are converted to strings before hashing.
 * 
 * @example
 * ```ts
 * import hash from "@ayonli/jsext/hash";
 * 
 * console.log(hash("Hello, World!")); // 4010631688
 * console.log(hash(new Uint8Array([1, 2, 3]))); // 193378021
 * ```
 */
export function hash(data: BufferSource): number {
    if (typeof data === "string") {
        return strHash(data);
    } else if (data instanceof ArrayBuffer) {
        const str = text(new Uint8Array(data));
        return strHash(str);
    } else if (data instanceof Uint8Array) {
        const str = text(data);
        return strHash(str);
    } else if (ArrayBuffer.isView(data)) {
        const bytes = new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
        const str = text(bytes);
        return strHash(str);
    } else {
        throw new TypeError("Unsupported data type");
    }
}

export async function toBytes(data: DataSource): Promise<Uint8Array> {
    let bin: Uint8Array;

    if (typeof data === "string") {
        bin = bytes(data);
    } else if (data instanceof ArrayBuffer) {
        bin = new Uint8Array(data);
    } else if (data instanceof Uint8Array) {
        bin = data;
    } else if (ArrayBuffer.isView(data)) {
        bin = new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
    } else if (typeof ReadableStream === "function" && data instanceof ReadableStream) {
        bin = new Uint8Array(await readAsArrayBuffer(data));
    } else if (typeof Blob === "function" && data instanceof Blob) {
        bin = new Uint8Array(await data.arrayBuffer());
    } else {
        throw new TypeError("Unsupported data type");
    }

    return bin;
}

export function sha1(data: DataSource): Promise<ArrayBuffer>;
export function sha1(data: DataSource, encoding: "hex" | "base64"): Promise<string>;
export async function sha1(
    data: DataSource,
    encoding: "hex" | "base64" | undefined = undefined
): Promise<string | ArrayBuffer> {
    const bytes = await toBytes(data);
    const hash = await crypto.subtle.digest("SHA-1", bytes);

    if (encoding === "hex") {
        return text(new Uint8Array(hash), "hex");
    } else if (encoding === "base64") {
        return text(new Uint8Array(hash), "base64");
    } else {
        return hash;
    }
}

export async function sha256(data: DataSource): Promise<ArrayBuffer>;
export async function sha256(data: DataSource, encoding: "hex" | "base64"): Promise<string>;
export async function sha256(
    data: DataSource,
    encoding: "hex" | "base64" | undefined = undefined
): Promise<string | ArrayBuffer> {
    const bytes = await toBytes(data);
    const hash = await crypto.subtle.digest("SHA-256", bytes);

    if (encoding === "hex") {
        return text(new Uint8Array(hash), "hex");
    } else if (encoding === "base64") {
        return text(new Uint8Array(hash), "base64");
    } else {
        return hash;
    }
}

export async function sha512(data: DataSource): Promise<ArrayBuffer>;
export async function sha512(data: DataSource, encoding: "hex" | "base64"): Promise<string>;
export async function sha512(
    data: DataSource,
    encoding: "hex" | "base64" | undefined = undefined
): Promise<string | ArrayBuffer> {
    const bytes = await toBytes(data);
    const hash = await crypto.subtle.digest("SHA-512", bytes);

    if (encoding === "hex") {
        return text(new Uint8Array(hash), "hex");
    } else if (encoding === "base64") {
        return text(new Uint8Array(hash), "base64");
    } else {
        return hash;
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
    const keyBuffer = await crypto.subtle.importKey("raw", bytes(key), {
        name: "HMAC",
        hash: {
            "sha1": "SHA-1",
            "sha256": "SHA-256",
            "sha512": "SHA-512",
        }[algorithm],
    }, false, ["sign"]);
    const dataBytes = await toBytes(data);
    const hash = await crypto.subtle.sign("HMAC", keyBuffer, dataBytes);

    if (encoding === "hex") {
        return text(new Uint8Array(hash), "hex");
    } else if (encoding === "base64") {
        return text(new Uint8Array(hash), "base64");
    } else {
        return hash;
    }
}
