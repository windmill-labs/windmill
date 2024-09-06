import { type LevelName, type LogLevel } from "./levels.js";
import type { LogRecord } from "./logger.js";
export type FormatterFunction = (logRecord: LogRecord) => string;
export interface BaseHandlerOptions {
    formatter?: FormatterFunction;
}
export declare abstract class BaseHandler {
    #private;
    formatter: FormatterFunction;
    constructor(levelName: LevelName, options?: BaseHandlerOptions);
    get level(): LogLevel;
    set level(level: LogLevel);
    get levelName(): LevelName;
    set levelName(levelName: LevelName);
    handle(logRecord: LogRecord): void;
    format(logRecord: LogRecord): string;
    abstract log(msg: string): void;
    setup(): void;
    destroy(): void;
    [Symbol.dispose](): void;
}
//# sourceMappingURL=base_handler.d.ts.map