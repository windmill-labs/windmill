import type { GenericFunction } from "./logger.js";
/** Log with error level, using default logger. */
export declare function error<T>(msg: () => T, ...args: unknown[]): T | undefined;
export declare function error<T>(msg: T extends GenericFunction ? never : T, ...args: unknown[]): T;
//# sourceMappingURL=error.d.ts.map