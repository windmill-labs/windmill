"use strict";
///<reference path="../lib.deno.d.ts" />
Object.defineProperty(exports, "__esModule", { value: true });
exports.env = void 0;
exports.env = {
    get(key) {
        assertValidKey(key);
        return process.env[key];
    },
    set(key, value) {
        assertValidKey(key);
        assertValidValue(value);
        process.env[key] = value;
    },
    has(key) {
        assertValidKey(key);
        return key in process.env;
    },
    delete(key) {
        assertValidKey(key);
        delete process.env[key];
    },
    // @ts-expect-error https://github.com/denoland/deno/issues/10267
    toObject() {
        return { ...process.env };
    },
};
const invalidKeyChars = ["=", "\0"].map((c) => c.charCodeAt(0));
const invalidValueChar = "\0".charCodeAt(0);
function assertValidKey(key) {
    if (key.length === 0) {
        throw new TypeError("Key is an empty string.");
    }
    for (let i = 0; i < key.length; i++) {
        if (invalidKeyChars.includes(key.charCodeAt(i))) {
            const char = key.charCodeAt(i) === "\0".charCodeAt(0) ? "\\0" : key[i];
            throw new TypeError(`Key contains invalid characters: "${char}"`);
        }
    }
}
function assertValidValue(value) {
    for (let i = 0; i < value.length; i++) {
        if (value.charCodeAt(i) === invalidValueChar) {
            throw new TypeError('Value contains invalid characters: "\\0"');
        }
    }
}
