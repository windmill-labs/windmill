import type { LevelName, LogLevel } from "./levels.js";
import type { BaseHandler } from "./base_handler.js";
export type GenericFunction = (...args: any[]) => any;
export interface LogRecordOptions {
    msg: string;
    args: unknown[];
    level: LogLevel;
    loggerName: string;
}
export declare class LoggerConfig {
    level?: LevelName;
    handlers?: string[];
}
export interface LogConfig {
    handlers?: {
        [name: string]: BaseHandler;
    };
    loggers?: {
        [name: string]: LoggerConfig;
    };
}
/**
 * An object that encapsulates provided message and arguments as well some
 * metadata that can be later used when formatting a message.
 */
export declare class LogRecord {
    #private;
    readonly msg: string;
    readonly level: number;
    readonly levelName: string;
    readonly loggerName: string;
    constructor(options: LogRecordOptions);
    get args(): unknown[];
    get datetime(): Date;
}
export interface LoggerOptions {
    handlers?: BaseHandler[];
}
export declare class Logger {
    #private;
    handlers: BaseHandler[];
    constructor(loggerName: string, levelName: LevelName, options?: LoggerOptions);
    /** Use this to retrieve the current numeric log level. */
    get level(): LogLevel;
    /** Use this to set the numeric log level. */
    set level(level: LogLevel);
    get levelName(): LevelName;
    set levelName(levelName: LevelName);
    get loggerName(): string;
    asString(data: unknown, isProperty?: boolean): string;
    debug<T>(msg: () => T, ...args: unknown[]): T | undefined;
    debug<T>(msg: T extends GenericFunction ? never : T, ...args: unknown[]): T;
    info<T>(msg: () => T, ...args: unknown[]): T | undefined;
    info<T>(msg: T extends GenericFunction ? never : T, ...args: unknown[]): T;
    warn<T>(msg: () => T, ...args: unknown[]): T | undefined;
    warn<T>(msg: T extends GenericFunction ? never : T, ...args: unknown[]): T;
    error<T>(msg: () => T, ...args: unknown[]): T | undefined;
    error<T>(msg: T extends GenericFunction ? never : T, ...args: unknown[]): T;
    critical<T>(msg: () => T, ...args: unknown[]): T | undefined;
    critical<T>(msg: T extends GenericFunction ? never : T, ...args: unknown[]): T;
}
//# sourceMappingURL=logger.d.ts.map