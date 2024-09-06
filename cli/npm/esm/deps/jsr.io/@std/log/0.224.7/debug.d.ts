import type { GenericFunction } from "./logger.js";
/** Log with debug level, using default logger. */
export declare function debug<T>(msg: () => T, ...args: unknown[]): T | undefined;
export declare function debug<T>(msg: T extends GenericFunction ? never : T, ...args: unknown[]): T;
//# sourceMappingURL=debug.d.ts.map