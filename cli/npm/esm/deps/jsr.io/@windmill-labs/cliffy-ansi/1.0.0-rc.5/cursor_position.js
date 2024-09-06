import { readSync } from "../../cliffy-internal/1.0.0-rc.5/runtime/read_sync.js";
import { setRaw } from "../../cliffy-internal/1.0.0-rc.5/runtime/set_raw.js";
import { writeSync } from "../../cliffy-internal/1.0.0-rc.5/runtime/write_sync.js";
import { cursorPosition } from "./ansi_escapes.js";
const encoder = new TextEncoder();
const decoder = new TextDecoder();
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
export function getCursorPosition({ reader = { readSync, setRaw }, writer = { writeSync }, } = {}) {
    const data = new Uint8Array(8);
    reader.setRaw(true);
    writer.writeSync(encoder.encode(cursorPosition));
    reader.readSync(data);
    reader.setRaw(false);
    const [y, x] = decoder
        .decode(data)
        .match(/\[(\d+);(\d+)R/)
        ?.slice(1, 3)
        .map(Number) ?? [0, 0];
    return { x, y };
}
