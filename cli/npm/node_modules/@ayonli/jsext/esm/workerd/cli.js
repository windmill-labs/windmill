export { args, charWidth, getWindowSize, isTTY, isTypingInput, isWSL, lockStdin, moveLeftBy, moveRightBy, parseArgs, quote, readStdin, stringWidth, writeStdout, writeStdoutSync } from '../cli/common.js';
export { ControlKeys, ControlSequences, FunctionKeys, NavigationKeys } from '../cli/constants.js';

async function run(cmd, args) {
    throw new Error("Unsupported runtime");
}
async function powershell(script) {
    throw new Error("Unsupported runtime");
}
async function sudo(cmd, args, options = {}) {
    throw new Error("Unsupported runtime");
}
async function which(cmd) {
    throw new Error("Unsupported runtime");
}
async function edit(filename) {
    throw new Error("Unsupported runtime");
}

export { edit, powershell, run, sudo, which };
//# sourceMappingURL=cli.js.map
