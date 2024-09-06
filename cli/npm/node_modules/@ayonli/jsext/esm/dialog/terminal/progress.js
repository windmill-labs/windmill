import bytes, { equals } from '../../bytes.js';
import '../../string/constants.js';
import '../../env.js';
import '../../external/event-target-polyfill/index.js';
import '../../path.js';
import { ControlKeys, ControlSequences } from '../../cli/constants.js';
import { writeStdoutSync, getWindowSize, stringWidth } from '../../cli/common.js';

const { CTRL_C, ESC, LF } = ControlKeys;
const { CLR } = ControlSequences;
const ongoingIndicators = [
    "      ",
    "=     ",
    "==    ",
    "===   ",
    " ===  ",
    "  === ",
    "   ===",
    "    ==",
    "     =",
    "      ",
];
async function handleTerminalProgress(message, fn, options) {
    const { signal, abort, listenForAbort } = options;
    let lastMessage = message;
    let lastPosition = 0;
    let lastPercent = undefined;
    const renderSimpleBar = (position = undefined) => {
        position !== null && position !== void 0 ? position : (position = lastPosition++);
        const ongoingIndicator = ongoingIndicators[position];
        writeStdoutSync(CLR);
        writeStdoutSync(bytes(`${lastMessage} [${ongoingIndicator}]`));
        if (lastPosition === ongoingIndicators.length) {
            lastPosition = 0;
        }
    };
    const renderPercentageBar = (percent) => {
        const { width } = getWindowSize();
        const percentage = percent + "%";
        const barWidth = width - stringWidth(lastMessage) - percentage.length - 5;
        const filled = "".padStart(Math.floor(barWidth * percent / 100), "#");
        const empty = "".padStart(barWidth - filled.length, "-");
        writeStdoutSync(CLR);
        writeStdoutSync(bytes(`${lastMessage} [${filled}${empty}] ${percentage}`));
    };
    renderSimpleBar();
    const waitingTimer = setInterval(renderSimpleBar, 200);
    const set = (state) => {
        if (signal.aborted) {
            return;
        }
        if (state.message) {
            lastMessage = state.message;
        }
        if (state.percent !== undefined) {
            lastPercent = state.percent;
        }
        if (lastPercent !== undefined) {
            renderPercentageBar(lastPercent);
            clearInterval(waitingTimer);
        }
        else if (state.message) {
            renderSimpleBar(lastPosition);
        }
    };
    const nodeReader = typeof Deno === "object" ? null : (buf) => {
        if (equals(buf, ESC) || equals(buf, CTRL_C)) {
            abort === null || abort === void 0 ? void 0 : abort();
        }
    };
    const denoReader = typeof Deno === "object" ? Deno.stdin.readable.getReader() : null;
    if (abort) {
        if (nodeReader) {
            process.stdin.on("data", nodeReader);
        }
        else if (denoReader) {
            (async () => {
                while (true) {
                    try {
                        const { done, value } = await denoReader.read();
                        if (done || equals(value, ESC) || equals(value, CTRL_C)) {
                            signal.aborted || abort();
                            break;
                        }
                    }
                    catch (_a) {
                        signal.aborted || abort();
                        break;
                    }
                }
                denoReader.releaseLock();
            })();
        }
    }
    let job = fn(set, signal);
    if (listenForAbort) {
        job = Promise.race([job, listenForAbort()]);
    }
    try {
        return await job;
    }
    finally {
        writeStdoutSync(LF);
        clearInterval(waitingTimer);
        if (nodeReader) {
            process.stdin.off("data", nodeReader);
        }
        else if (denoReader) {
            denoReader.releaseLock();
        }
    }
}

export { handleTerminalProgress };
//# sourceMappingURL=progress.js.map
