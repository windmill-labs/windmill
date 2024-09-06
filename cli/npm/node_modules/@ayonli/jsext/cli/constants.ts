import bytes, { ByteArray } from "../bytes.ts";

export namespace ControlKeys {
    /** ^I - Tab */
    export const TAB: ByteArray = bytes("\t");
    /** ^J - Enter on Linux */
    export const LF: ByteArray = bytes("\n");
    /** ^M - Enter on macOS and Windows (CRLF) */
    export const CR: ByteArray = bytes("\r");
    /** ^H - Backspace on Linux and Windows */
    export const BS: ByteArray = bytes("\b");
    /** ^? - Backspace on macOS */
    export const DEL: ByteArray = bytes([0x7f]);
    /** ^[ - Escape */
    export const ESC: ByteArray = bytes([0x1b]);
    /** ^C - Cancel */
    export const CTRL_C: ByteArray = bytes([0x03]);
    /** ^A - Start of line */
    export const CTRL_A: ByteArray = bytes([0x01]);
    /** ^E - End of line */
    export const CTRL_E: ByteArray = bytes([0x05]);
}

export namespace NavigationKeys {
    export const UP: ByteArray = bytes("\u001b[A");
    export const DOWN: ByteArray = bytes("\u001b[B");
    export const LEFT: ByteArray = bytes("\u001b[D");
    export const RIGHT: ByteArray = bytes("\u001b[C");
    export const HOME: ByteArray = bytes("\u001b[H");
    export const END: ByteArray = bytes("\u001b[F");
    export const PAGE_UP: ByteArray = bytes("\u001b[5~");
    export const PAGE_DOWN: ByteArray = bytes("\u001b[6~");
    export const INSERT: ByteArray = bytes("\u001b[2~");
    export const DELETE: ByteArray = bytes("\u001b[3~");
}

export namespace FunctionKeys {
    export const F1: ByteArray = bytes("\u001bOP");
    export const F2: ByteArray = bytes("\u001bOQ");
    export const F3: ByteArray = bytes("\u001bOR");
    export const F4: ByteArray = bytes("\u001bOS");
    export const F5: ByteArray = bytes("\u001b[15~");
    export const F6: ByteArray = bytes("\u001b[17~");
    export const F7: ByteArray = bytes("\u001b[18~");
    export const F8: ByteArray = bytes("\u001b[19~");
    export const F9: ByteArray = bytes("\u001b[20~");
    export const F10: ByteArray = bytes("\u001b[21~");
    export const F11: ByteArray = bytes("\u001b[23~");
    export const F12: ByteArray = bytes("\u001b[24~");
}

export namespace ControlSequences {
    /** Clear the current line */
    export const CLR: ByteArray = bytes("\r\u001b[K");
    /** Clear the right side of the cursor */
    export const CLR_RIGHT: ByteArray = bytes("\u001b[0K");
    /** Clear the left side of the cursor */
    export const CLR_LEFT: ByteArray = bytes("\u001b[1K");
    /** Clear the screen from the cursor down */
    export const CLR_DOWN: ByteArray = bytes("\u001b[0J");
    /** Clear the screen from the cursor up */
    export const CLR_UP: ByteArray = bytes("\u001b[1J");
    /** Clear the entire screen */
    export const CLR_SCREEN: ByteArray = bytes("\u001b[2J");
    /** Hide the cursor */
    export const HIDE_CURSOR: ByteArray = bytes("\u001b[?25l");
    /** Show the cursor */
    export const SHOW_CURSOR: ByteArray = bytes("\u001b[?25h");
    /** Enable mouse tracking */
    export const MOUSE_ON: ByteArray = bytes("\u001b[?1000h");
    /** Disable mouse tracking */
    export const MOUSE_OFF: ByteArray = bytes("\u001b[?1000l");
    /** Enable mouse wheel scrolling */
    export const MOUSE_WHEEL_ON: ByteArray = bytes("\u001b[?1015h");
    /** Disable mouse wheel scrolling */
    export const MOUSE_WHEEL_OFF: ByteArray = bytes("\u001b[?1015l");
    /** Enable bracketed paste mode */
    export const PASTE_ON: ByteArray = bytes("\u001b[?2004h");
    /** Disable bracketed paste mode */
    export const PASTE_OFF: ByteArray = bytes("\u001b[?2004l");
    /** Enable line wrapping */
    export const WRAP_ON: ByteArray = bytes("\u001b[?7h");
    /** Disable line wrapping */
    export const WRAP_OFF: ByteArray = bytes("\u001b[?7l");
    /** Enable alternate screen buffer */
    export const ALT_BUFFER_ON: ByteArray = bytes("\u001b[?1049h");
    /** Disable alternate screen buffer */
    export const ALT_BUFFER_OFF: ByteArray = bytes("\u001b[?1049l");
}

export const PowerShellCommands = [
    "ac",
    "asnp",
    "cat",
    "cd",
    "chdir",
    "clc",
    "clear",
    "clhy",
    "cli",
    "clp",
    "cls",
    "clv",
    "cnsn",
    "compare",
    "copy",
    "cp",
    "cpi",
    "cpp",
    "curl",
    "cvpa",
    "dbp",
    "del",
    "diff",
    "dir",
    "dnsn",
    "ebp",
    "echo",
    "epal",
    "epcsv",
    "epsn",
    "erase",
    "etsn",
    "exsn",
    "fc",
    "fl",
    "foreach",
    "ft",
    "fw",
    "gal",
    "gbp",
    "gc",
    "gci",
    "gcm",
    "gcs",
    "gdr",
    "ghy",
    "gi",
    "gjb",
    "gl",
    "gm",
    "gmo",
    "gp",
    "gps",
    "group",
    "gsn",
    "gsnp",
    "gsv",
    "gu",
    "gv",
    "gwmi",
    "h",
    "history",
    "icm",
    "iex",
    "ihy",
    "ii",
    "ipal",
    "ipcsv",
    "ipmo",
    "ipsn",
    "irm",
    "ise",
    "iwmi",
    "iwr",
    "kill",
    "lp",
    "ls",
    "man", "help",
    "md", "mkdir",
    "measure",
    "mi",
    "mount",
    "move",
    "mp",
    "mv",
    "nal",
    "ndr",
    "ni",
    "nmo",
    "npssc",
    "nsn",
    "nv",
    "ogv",
    "oh",
    "popd",
    "ps",
    "pushd",
    "pwd",
    "r",
    "rbp",
    "rcjb",
    "rcsn",
    "rd",
    "rdr",
    "ren",
    "ri",
    "rjb",
    "rm",
    "rmdir",
    "rmo",
    "rni",
    "rnp",
    "rp",
    "rsn",
    "rsnp",
    "rujb",
    "rv",
    "rvpa",
    "rwmi",
    "sajb",
    "sal",
    "saps",
    "sasv",
    "sbp",
    "sc",
    "select",
    "set",
    "shcm",
    "si"
];
