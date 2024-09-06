import bytes, { concat, equals } from "../../bytes.ts";
import { chars } from "../../string.ts";
import {
    ControlKeys,
    ControlSequences,
    NavigationKeys,
    isTypingInput,
    moveLeftBy,
    moveRightBy,
    readStdin,
    writeStdout,
} from "../../cli.ts";

const { BS, CTRL_A, CTRL_C, CTRL_E, CR, DEL, ESC, LF } = ControlKeys;
const { UP, DOWN, LEFT, RIGHT } = NavigationKeys;
const { CLR_RIGHT } = ControlSequences;

function getMasks(mask: string, length: number): string {
    return new Array<string>(length).fill(mask).join("");
}

export default async function question(message: string, options: {
    defaultValue?: string | undefined;
    mask?: string | undefined;
} = {}): Promise<string | null> {
    const { defaultValue = "", mask } = options;
    const buf: string[] = [];
    let cursor = 0;

    await writeStdout(bytes(message));

    if (defaultValue) {
        const _chars = chars(defaultValue);
        buf.push(..._chars);
        cursor += _chars.length;

        if (mask === undefined) {
            await writeStdout(bytes(defaultValue));
        } else if (mask) {
            await writeStdout(bytes(getMasks(mask, _chars.length)));
        }
    }

    while (true) {
        const input = await readStdin();

        if (!input.length || equals(input, UP) || equals(input, DOWN)) {
            continue;
        } else if (equals(input, LEFT)) {
            if (cursor > 0) {
                const char = buf[--cursor]!;

                if (mask === undefined) {
                    await moveLeftBy(char);
                } else if (mask) {
                    await moveLeftBy(mask);
                }
            }
        } else if (equals(input, RIGHT)) {
            if (cursor < buf.length) {
                const char = buf[cursor++]!;

                if (mask === undefined) {
                    await moveRightBy(char);
                } else if (mask) {
                    await moveRightBy(mask);
                }
            }
        } else if (equals(input, CTRL_A)) {
            const left = buf.slice(0, cursor);

            if (left.length) {
                cursor = 0;

                if (mask === undefined) {
                    await moveLeftBy(left.join(""));
                } else if (mask) {
                    await moveLeftBy(getMasks(mask, left.length));
                }
            }
        } else if (equals(input, CTRL_E)) {
            const right = buf.slice(cursor);

            if (right.length) {
                cursor = buf.length;

                if (mask === undefined) {
                    await moveRightBy(right.join(""));
                } else if (mask) {
                    await moveRightBy(getMasks(mask, right.length));
                }
            }
        } else if (equals(input, ESC) || equals(input, CTRL_C)) {
            await writeStdout(LF);
            return null;
        } else if (equals(input, CR) || equals(input, LF)) {
            await writeStdout(LF);
            return buf.join("");
        } else if (equals(input, BS) || equals(input, DEL)) {
            if (cursor > 0) {
                cursor--;
                const [char] = buf.splice(cursor, 1);
                const rest = buf.slice(cursor);

                if (mask === undefined) {
                    await moveLeftBy(char!);
                    await writeStdout(CLR_RIGHT);

                    if (rest.length) {
                        const output = rest.join("");
                        await writeStdout(bytes(output));
                        await moveLeftBy(output);
                    }
                } else if (mask) {
                    await moveLeftBy(mask);
                    await writeStdout(CLR_RIGHT);

                    if (rest.length) {
                        const output = getMasks(mask, rest.length);
                        await writeStdout(bytes(output));
                        await moveLeftBy(output);
                    }
                }
            }
        } else if (isTypingInput(input)) {
            const _chars = chars(String(input));

            if (cursor === buf.length) {
                buf.push(..._chars);
                cursor += _chars.length;

                if (mask === undefined) {
                    await writeStdout(input);
                } else if (mask) {
                    await writeStdout(bytes(getMasks(mask, _chars.length)));
                }
            } else {
                buf.splice(cursor, 0, ..._chars);
                cursor += _chars.length;

                if (mask === undefined) {
                    const rest = buf.slice(cursor).join("");

                    await writeStdout(concat(input, bytes(rest)));
                    await moveLeftBy(rest);
                } else if (mask) {
                    const output = getMasks(mask, _chars.length);
                    const rest = getMasks(mask, buf.slice(cursor).length);

                    await writeStdout(bytes(output + rest));
                    await moveLeftBy(rest);
                }
            }
        }
    }
}
