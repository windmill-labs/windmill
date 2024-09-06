import * as dntShim from "../../../../../_dnt.shims.js";
import type { ReaderSync, WriterSync } from "../../../@std/io/0.224.7/types.js";
/** Cursor position. */
export interface Cursor {
    x: number;
    y: number;
}
/** Cursor position options. */
export interface CursorPositionOptions {
    writer?: WriterSync;
    reader?: ReaderSync & {
        setRaw(mode: boolean, options?: dntShim.Deno.SetRawOptions): void;
    };
}
/**
 * Get cursor position.
 *
 * @param options  Options.
 *
 * ```ts
 * import { Cursor, getCursorPosition } from "@cliffy/ansi/cursor-position";
 *
 * const cursor: Cursor = getCursorPosition();
 * console.log(cursor); // { x: 0, y: 14}
 * ```
 */
export declare function getCursorPosition({ reader, writer, }?: CursorPositionOptions): Cursor;
//# sourceMappingURL=cursor_position.d.ts.map