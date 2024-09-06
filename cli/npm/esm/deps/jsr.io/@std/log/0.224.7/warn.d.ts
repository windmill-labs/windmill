import type { GenericFunction } from "./logger.js";
/** Log with warning level, using default logger. */
export declare function warn<T>(msg: () => T, ...args: unknown[]): T | undefined;
export declare function warn<T>(msg: T extends GenericFunction ? never : T, ...args: unknown[]): T;
//# sourceMappingURL=warn.d.ts.map