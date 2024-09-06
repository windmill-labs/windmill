import type { GenericFunction } from "./logger.js";
/** Log with info level, using default logger. */
export declare function info<T>(msg: () => T, ...args: unknown[]): T | undefined;
export declare function info<T>(msg: T extends GenericFunction ? never : T, ...args: unknown[]): T;
//# sourceMappingURL=info.d.ts.map