import bytes from './bytes.js';
import { isDeno, isNodeLike } from './env.js';
import { hash, sha1 as sha1$1, sha256 as sha256$1, sha512 as sha512$1, hmac as hmac$1, toBytes } from './hash/web.js';

/**
 * Simplified hash functions for various data types, based on the Web Crypto API
 * and `crypto` package in Node.js.
 * @module
 */
async function nodeHash(algorithm, data, encoding = undefined) {
    const crypto = await import('node:crypto');
    const bytes = await toBytes(data);
    const hash = crypto.createHash(algorithm);
    hash.update(bytes);
    if (encoding) {
        return hash.digest(encoding);
    }
    else {
        const result = hash.digest();
        // Truncate the buffer to the actual byte length so it's consistent with the web API.
        return result.buffer.slice(result.byteOffset, result.byteOffset + result.byteLength);
    }
}
async function sha1(data, encoding = undefined) {
    if (typeof crypto === "object") {
        return encoding ? sha1$1(data, encoding) : sha1$1(data);
    }
    else if (isDeno || isNodeLike) {
        return nodeHash("sha1", data, encoding);
    }
    else {
        throw new Error("Unsupported runtime");
    }
}
async function sha256(data, encoding = undefined) {
    if (typeof crypto === "object") {
        return encoding ? sha256$1(data, encoding) : sha256$1(data);
    }
    else if (isDeno || isNodeLike) {
        return nodeHash("sha256", data, encoding);
    }
    else {
        throw new Error("Unsupported runtime");
    }
}
async function sha512(data, encoding = undefined) {
    if (typeof crypto === "object") {
        return encoding ? sha512$1(data, encoding) : sha512$1(data);
    }
    else if (isDeno || isNodeLike) {
        return nodeHash("sha512", data, encoding);
    }
    else {
        throw new Error("Unsupported runtime");
    }
}
async function md5(data, encoding = undefined) {
    if (isDeno || isNodeLike) {
        return nodeHash("md5", data, encoding);
    }
    else {
        throw new Error("Unsupported runtime");
    }
}
async function hmac(algorithm, key, data, encoding = undefined) {
    if (typeof crypto === "object") {
        return encoding ? hmac$1(algorithm, key, data, encoding) : hmac$1(algorithm, key, data);
    }
    else if (isDeno || isNodeLike) {
        const crypto = await import('node:crypto');
        const binary = await toBytes(data);
        const hash = crypto.createHmac(algorithm, bytes(key));
        hash.update(binary);
        if (encoding) {
            return hash.digest(encoding);
        }
        else {
            const result = hash.digest();
            // Truncate the buffer to the actual byte length so it's consistent with the web API.
            return result.buffer.slice(result.byteOffset, result.byteOffset + result.byteLength);
        }
    }
    else {
        throw new Error("Unsupported runtime");
    }
}

export { hash as default, hmac, md5, sha1, sha256, sha512 };
//# sourceMappingURL=hash.js.map
