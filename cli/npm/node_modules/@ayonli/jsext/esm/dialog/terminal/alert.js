import { escape } from './util.js';
import { dedent } from '../../string.js';
import { platform } from '../../runtime.js';
import { run, powershell, which } from '../../cli.js';
import question from './question.js';
import { isWSL, lockStdin } from '../../cli/common.js';

function createAppleScript(message) {
    return dedent `
        tell application (path to frontmost application as text)
            display dialog "${escape(message)}" with title "Alert" buttons {"OK"} default button "OK"
        end
        `;
}
function createPowerShellScript(message) {
    return dedent `
        Add-Type -AssemblyName PresentationFramework
        [System.Windows.MessageBox]::Show("${escape(message)}", "Alert")
        `;
}
async function alertInTerminal(message, options = {}) {
    if ((options === null || options === void 0 ? void 0 : options.gui) && platform() === "darwin") {
        const { code, stderr } = await run("osascript", [
            "-e",
            createAppleScript(message)
        ]);
        if (code) {
            throw new Error(stderr);
        }
    }
    else if ((options === null || options === void 0 ? void 0 : options.gui) && (platform() === "windows" || isWSL())) {
        const { code, stderr } = await powershell(createPowerShellScript(message));
        if (code) {
            throw new Error(stderr);
        }
    }
    else if ((options === null || options === void 0 ? void 0 : options.gui) && (platform() === "linux" || await which("zenity"))) {
        const args = [
            "--info",
            "--title", "Alert",
            "--width", "365",
        ];
        if (message) {
            args.push("--text", message);
        }
        const { code, stderr } = await run("zenity", args);
        if (code && code !== 1) {
            throw new Error(stderr);
        }
    }
    else {
        await lockStdin(() => question(message + " [Enter] "));
    }
}

export { alertInTerminal as default };
//# sourceMappingURL=alert.js.map
