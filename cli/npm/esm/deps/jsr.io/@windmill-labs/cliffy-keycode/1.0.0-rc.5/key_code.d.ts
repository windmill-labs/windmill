/** KeyCode object. */
export interface KeyCode {
    /** Key name. */
    name?: string;
    /** Key sequence. */
    sequence?: string;
    /** Key code. */
    code?: string;
    /** Indicates if the ctrl key is pressed. */
    ctrl?: boolean;
    /** Indicates if the meta key is pressed. */
    meta?: boolean;
    /** Indicates if the shift key is pressed. */
    shift?: boolean;
    /** Key string value. */
    char?: string;
}
/**
 * Parse ansi escape sequence.
 *
 * @param data Ansi escape sequence.
 *
 * ```ts
 * import { parse } from "./mod.ts";
 *
 * parse("\x04\x18");
 * ```
 *
 * ```json
 * [
 *   KeyCode { name: "d", sequence: "\x04", ctrl: true, meta: false, shift: false },
 *   KeyCode { name: "x", sequence: "\x18", ctrl: true, meta: false, shift: false },
 * ]
 * ```
 */
export declare function parse(data: Uint8Array | string): KeyCode[];
//# sourceMappingURL=key_code.d.ts.map