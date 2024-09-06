import type { GenericFunction } from "./logger.js";
/** Log with critical level, using default logger. */
export declare function critical<T>(msg: () => T, ...args: unknown[]): T | undefined;
export declare function critical<T>(msg: T extends GenericFunction ? never : T, ...args: unknown[]): T;
//# sourceMappingURL=critical.d.ts.map