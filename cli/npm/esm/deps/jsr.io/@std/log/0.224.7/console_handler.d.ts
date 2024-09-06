import { type LevelName } from "./levels.js";
import type { LogRecord } from "./logger.js";
import { BaseHandler, type BaseHandlerOptions } from "./base_handler.js";
export interface ConsoleHandlerOptions extends BaseHandlerOptions {
    useColors?: boolean;
}
/**
 * This is the default logger. It will output color coded log messages to the
 * console via `console.log()`.
 */
export declare class ConsoleHandler extends BaseHandler {
    #private;
    constructor(levelName: LevelName, options?: ConsoleHandlerOptions);
    format(logRecord: LogRecord): string;
    applyColors(msg: string, level: number): string;
    log(msg: string): void;
}
//# sourceMappingURL=console_handler.d.ts.map