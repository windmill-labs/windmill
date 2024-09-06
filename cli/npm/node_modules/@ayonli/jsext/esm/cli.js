import { trimStart } from './string.js';
import { text } from './bytes.js';
import { isDeno, isNodeLike, isBun, isBrowserWindow, isSharedWorker, isDedicatedWorker } from './env.js';
import runtime, { platform as platform$1, env } from './runtime.js';
import { interop } from './module.js';
import { basename } from './path.js';
import { PowerShellCommands } from './cli/constants.js';
export { ControlKeys, ControlSequences, FunctionKeys, NavigationKeys } from './cli/constants.js';
import { isWSL, quote } from './cli/common.js';
export { args, charWidth, getWindowSize, isTTY, isTypingInput, lockStdin, moveLeftBy, moveRightBy, parseArgs, readStdin, stringWidth, writeStdout, writeStdoutSync } from './cli/common.js';

/**
 * Useful utility functions for interacting with the terminal.
 *
 * NOTE: Despite the name of this module, many of its functions can also be used
 * in the browser environment.
 * @module
 * @experimental
 */
/**
 * @deprecated use `runtime().tsSupport` from `@ayonli/jsext/runtime` module instead.
 */
const isTsRuntime = () => runtime().tsSupport;
/**
 * @deprecated import `platform` from `@ayonli/jsext/runtime` module instead.
 */
const platform = platform$1;
/**
 * Executes a command in the terminal and returns the exit code and outputs.
 *
 * In Windows, this function will use PowerShell to execute the command when
 * possible, which has a lot UNIX-like aliases/commands available, such as `ls`,
 * `cat`, `rm`, etc.
 *
 * @example
 * ```ts
 * import { run } from "@ayonli/jsext/cli";
 *
 * const { code, stdout, stderr } = await run("echo", ["Hello, World!"]);
 *
 * console.log(code); // 0
 * console.log(JSON.stringify(stdout)); // "Hello, World!\n"
 * console.log(JSON.stringify(stderr)); // ""
 * ```
 */
async function run(cmd, args) {
    const isWindows = platform() === "windows";
    const isWslPs = isWSL() && cmd.endsWith("powershell.exe");
    if (isDeno) {
        const { Buffer } = await import('node:buffer');
        const { decode } = interop(await import('iconv-lite'), false);
        const _cmd = isWindows && PowerShellCommands.includes(cmd)
            ? new Deno.Command("powershell", { args: ["-c", cmd, ...args.map(quote)] })
            : new Deno.Command(cmd, { args });
        const { code, stdout, stderr } = await _cmd.output();
        return {
            code,
            stdout: isWindows || isWslPs ? decode(Buffer.from(stdout), "cp936") : text(stdout),
            stderr: isWindows || isWslPs ? decode(Buffer.from(stderr), "cp936") : text(stderr),
        };
    }
    else if (isNodeLike) {
        const { spawn } = await import('child_process');
        const { decode } = await interop(import('iconv-lite'), false);
        const child = isWindows && PowerShellCommands.includes(cmd)
            ? spawn("powershell", ["-c", cmd, ...args.map(quote)])
            : spawn(cmd, args);
        const stdout = [];
        const stderr = [];
        child.stdout.on("data", chunk => {
            if (isWindows || isWslPs) {
                stdout.push(decode(chunk, "cp936"));
            }
            else {
                stdout.push(String(chunk));
            }
        });
        child.stderr.on("data", chunk => {
            if (isWindows || isWslPs) {
                stderr.push(decode(chunk, "cp936"));
            }
            else {
                stderr.push(String(chunk));
            }
        });
        const code = await new Promise((resolve, reject) => {
            child.once("exit", (code, signal) => {
                if (code === null && signal) {
                    resolve(1);
                }
                else {
                    resolve(code !== null && code !== void 0 ? code : 0);
                }
            }).once("error", reject);
        });
        return {
            code,
            stdout: stdout.join(""),
            stderr: stderr.join(""),
        };
    }
    else {
        throw new Error("Unsupported runtime");
    }
}
/**
 * Executes the script inside PowerShell as if they were typed at the PowerShell
 * command prompt.
 *
 * This function can also be called within Windows Subsystem for Linux to
 * directly interact with PowerShell.
 *
 * NOTE: This function is only available in Windows and Windows Subsystem for
 * Linux.
 *
 * @example
 * ```ts
 * import { powershell } from "@ayonli/jsext/cli";
 *
 * const cmd = "ls";
 * const {
 *     code,
 *     stdout,
 *     stderr,
 * } = await powershell(`Get-Command -Name ${cmd} | Select-Object -ExpandProperty Source`);
 * ```
 */
async function powershell(script) {
    let command = "powershell";
    if (isWSL()) {
        command = "/mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe";
    }
    return await run(command, ["-c", script]);
}
/**
 * Executes a command with elevated privileges using `sudo` (or UAC in Windows).
 *
 * @example
 * ```ts
 * import { sudo } from "@ayonli/jsext/cli";
 *
 * await sudo("apt", ["install", "build-essential"]);
 * ```
 */
async function sudo(cmd, args, options = {}) {
    const _platform = platform();
    if ((_platform !== "windows" && !(options === null || options === void 0 ? void 0 : options.gui)) ||
        (_platform === "linux" && !env("DISPLAY")) ||
        isWSL()) {
        return await run("sudo", [cmd, ...args]);
    }
    if (!["darwin", "windows", "linux"].includes(_platform)) {
        throw new Error("Unsupported platform");
    }
    const { exec } = await interop(import('sudo-prompt'));
    return await new Promise((resolve, reject) => {
        exec(`${cmd}` + (args.length ? ` ${args.map(quote).join(" ")}` : ""), {
            name: (options === null || options === void 0 ? void 0 : options.title) || (isDeno ? "Deno" : isBun ? "Bun" : "NodeJS"),
        }, (error, stdout, stderr) => {
            if (error) {
                reject(error);
            }
            else {
                let _stdout = String(stdout);
                if (_platform === "windows" && cmd === "echo" && _stdout.startsWith(`"`)) {
                    // In Windows CMD, the `echo` command will output the string
                    // with double quotes. We need to remove them.
                    let lastIndex = _stdout.lastIndexOf(`"`);
                    _stdout = _stdout.slice(1, lastIndex) + _stdout.slice(lastIndex + 1);
                }
                resolve({
                    code: 0,
                    stdout: _stdout,
                    stderr: String(stderr),
                });
            }
        });
    });
}
/**
 * Returns the path of the given command if it exists in the system,
 * otherwise returns `null`.
 *
 * This function is available in Windows as well.
 *
 * @example
 * ```ts
 * import { which } from "@ayonli/jsext/cli";
 *
 * const path = await which("node");
 *
 * console.log(path);
 * // e.g. "/usr/bin/node" in UNIX/Linux or "C:\\Program Files\\nodejs\\node.exe" in Windows
 * ```
 */
async function which(cmd) {
    if (platform() === "windows") {
        const { code, stdout } = await run("powershell", [
            "-Command",
            `Get-Command -Name ${cmd} | Select-Object -ExpandProperty Source`
        ]);
        return code ? null : stdout.trim();
    }
    else {
        const { code, stdout } = await run("which", [cmd]);
        return code ? null : stdout.trim();
    }
}
/**
 * Opens the given file in a text editor.
 *
 * The `filename` can include a line number by appending `:<number>` or `#L<number>`,
 * however, this feature is not supported by all editors.
 *
 * This function will try to open VS Code if available, otherwise it will try to
 * open the default editor or a preferred one, such as `vim` or `nano` when available.
 *
 * Some editor may hold the terminal until the editor is closed, while others may
 * return immediately. Anyway, the operation is asynchronous and the function will
 * not block the thread.
 *
 * In the browser, this function will always try to open the file in VS Code,
 * regardless of whether it's available or not.
 *
 * @example
 * ```ts
 * import { edit } from "@ayonli/jsext/cli";
 *
 * await edit("path/to/file.txt");
 *
 * await edit("path/to/file.txt:10"); // open the file at line 10
 * ```
 */
async function edit(filename) {
    const match = filename.match(/(:|#L)(\d+)/);
    let line;
    if (match) {
        line = Number(match[2]);
        filename = filename.slice(0, match.index);
    }
    if (isBrowserWindow) {
        window.open("vscode://file/" + trimStart(filename, "/") + (line ? `:${line}` : ""));
        return;
    }
    else if (isSharedWorker
        || isSharedWorker
        || (isDedicatedWorker && (["chrome", "firefox", "safari"]).includes(runtime().identity))) {
        throw new Error("Unsupported runtime");
    }
    const _platform = platform();
    const vscode = await which("code");
    if (vscode) {
        const args = line ? ["--goto", `${filename}:${line}`] : [filename];
        const { code, stderr } = await run(vscode, args);
        if (code)
            throw new Error(stderr || `Unable to open ${filename} in the editor.`);
        return;
    }
    else if (_platform === "darwin") {
        const { code, stderr } = await run("open", ["-t", filename]);
        if (code)
            throw new Error(stderr || `Unable to open ${filename} in the editor.`);
        return;
    }
    else if (_platform === "windows" || isWSL()) {
        const notepad = _platform === "windows"
            ? "notepad.exe"
            : "/mnt/c/Windows/System32/notepad.exe";
        const { code, stderr } = await run(notepad, [filename]);
        if (code)
            throw new Error(stderr || `Unable to open ${filename} in the editor.`);
        return;
    }
    let editor = env("EDITOR")
        || env("VISUAL")
        || (await which("gedit"))
        || (await which("kate"))
        || (await which("vim"))
        || (await which("vi"))
        || (await which("nano"));
    let args;
    if (!editor) {
        throw new Error("Cannot determine the editor to open.");
    }
    else {
        editor = basename(editor);
    }
    if (["gedit", "kate", "vim", "vi", "nano"].includes(editor)) {
        args = line ? [`+${line}`, filename] : [filename];
    }
    if (["vim", "vi", "nano"].includes(editor)) {
        if (await which("gnome-terminal")) {
            args = ["--", editor, ...args];
            editor = "gnome-terminal";
        }
        else {
            args = ["-e", `'${editor} ${args.map(quote).join(" ")}'`];
            editor = (await which("konsole"))
                || (await which("xfce4-terminal"))
                || (await which("deepin-terminal"))
                || (await which("xterm"));
        }
        if (!editor) {
            throw new Error("Cannot determine the terminal to open.");
        }
    }
    else {
        args = [filename];
    }
    const { code, stderr } = await run(editor, args);
    if (code)
        throw new Error(stderr || `Unable to open ${filename} in the editor.`);
}

export { edit, isTsRuntime, isWSL, platform, powershell, quote, run, sudo, which };
//# sourceMappingURL=cli.js.map
