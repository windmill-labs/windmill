"use strict";
// constants.ts
/**
 * Cross-platform path separator constants
 * Compatible with both Node.js and Deno environments
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.DELIMITER = exports.SEP = void 0;
/**
 * Detects if the current platform is Windows by checking various environment indicators.
 * Uses multiple fallback methods to ensure compatibility across Node.js, Deno, and browser environments.
 *
 * @returns True if running on Windows, false otherwise
 */
const isWindows = (() => {
    // Try Node.js process.platform first (most reliable)
    if (typeof process !== "undefined" && process.platform) {
        return process.platform === "win32";
    }
    // Try Deno specific detection via globalThis
    if (typeof globalThis !== "undefined" && "Deno" in globalThis) {
        return globalThis.Deno?.build?.os === "windows";
    }
    // Try navigator.platform (browser/some Deno contexts)
    // @ts-ignore
    if (typeof navigator !== "undefined" && navigator.platform) {
        // @ts-ignore
        return navigator.platform.toLowerCase().includes("win");
    }
    // Fallback - assume Unix-like (safest default)
    return false;
})();
/**
 * Path separator constant - equivalent to Deno's SEPARATOR or Node's path.sep
 * On Windows: "\"
 * On Unix-like systems: "/"
 */
exports.SEP = isWindows ? "\\" : "/";
/**
 * Path delimiter constant for environment variables like PATH
 * On Windows: ";"
 * On Unix-like systems: ":"
 */
exports.DELIMITER = isWindows ? ";" : ":";
