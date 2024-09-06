import * as dntShim from "../../../../../_dnt.shims.js";
import { type LevelName } from "./levels.js";
import type { LogRecord } from "./logger.js";
import { BaseHandler, type BaseHandlerOptions } from "./base_handler.js";
import { bufSymbol, encoderSymbol, filenameSymbol, fileSymbol, modeSymbol, openOptionsSymbol, pointerSymbol } from "./_file_handler_symbols.js";
export type LogMode = "a" | "w" | "x";
export interface FileHandlerOptions extends BaseHandlerOptions {
    filename: string;
    /**
     * @default {"a"}
     */
    mode?: LogMode;
    /**
     * Buffer size for writing log messages to file, in bytes.
     *
     * @default {4096}
     */
    bufferSize?: number;
}
/**
 * This handler will output to a file using an optional mode (default is `a`,
 * e.g. append). The file will grow indefinitely. It uses a buffer for writing
 * to file. Logs can be manually flushed with `fileHandler.flush()`. Log
 * messages with a log level greater than error are immediately flushed. Logs
 * are also flushed on process completion.
 *
 * Behavior of the log modes is as follows:
 *
 * - `'a'` - Default mode. Appends new log messages to the end of an existing log
 *   file, or create a new log file if none exists.
 * - `'w'` - Upon creation of the handler, any existing log file will be removed
 *   and a new one created.
 * - `'x'` - This will create a new log file and throw an error if one already
 *   exists.
 *
 * This handler requires `--allow-write` permission on the log file.
 */
export declare class FileHandler extends BaseHandler {
    #private;
    [fileSymbol]: dntShim.Deno.FsFile | undefined;
    [bufSymbol]: Uint8Array;
    [pointerSymbol]: number;
    [filenameSymbol]: string;
    [modeSymbol]: LogMode;
    [openOptionsSymbol]: dntShim.Deno.OpenOptions;
    [encoderSymbol]: TextEncoder;
    constructor(levelName: LevelName, options: FileHandlerOptions);
    setup(): void;
    handle(logRecord: LogRecord): void;
    log(msg: string): void;
    flush(): void;
    destroy(): void;
}
//# sourceMappingURL=file_handler.d.ts.map