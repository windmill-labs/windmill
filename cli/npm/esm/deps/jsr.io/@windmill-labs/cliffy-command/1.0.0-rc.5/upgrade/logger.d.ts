import type { Spinner } from "./spinner.js";
export interface Logger {
    log(...data: Array<unknown>): void;
    info(...data: Array<unknown>): void;
    warn(...data: Array<unknown>): void;
    error(...data: Array<unknown>): void;
}
export interface LoggerOptions {
    spinner?: Spinner;
    verbose?: boolean;
}
export declare function createLogger({ spinner, verbose }?: LoggerOptions): Logger;
//# sourceMappingURL=logger.d.ts.map