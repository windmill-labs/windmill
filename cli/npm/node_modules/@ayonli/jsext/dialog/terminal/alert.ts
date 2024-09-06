import { escape } from "./util.ts";
import { dedent } from "../../string.ts";
import { platform } from "../../runtime.ts";
import { isWSL, lockStdin, powershell, run, which } from "../../cli.ts";
import question from "./question.ts";

function createAppleScript(message: string) {
    return dedent`
        tell application (path to frontmost application as text)
            display dialog "${escape(message)}" with title "Alert" buttons {"OK"} default button "OK"
        end
        `;
}

function createPowerShellScript(message: string) {
    return dedent`
        Add-Type -AssemblyName PresentationFramework
        [System.Windows.MessageBox]::Show("${escape(message)}", "Alert")
        `;
}

export default async function alertInTerminal(message: string, options: {
    gui?: boolean;
} = {}): Promise<void> {
    if (options?.gui && platform() === "darwin") {
        const { code, stderr } = await run("osascript", [
            "-e",
            createAppleScript(message)
        ]);

        if (code) {
            throw new Error(stderr);
        }
    } else if (options?.gui && (platform() === "windows" || isWSL())) {
        const { code, stderr } = await powershell(
            createPowerShellScript(message)
        );

        if (code) {
            throw new Error(stderr);
        }
    } else if (options?.gui && (platform() === "linux" || await which("zenity"))) {
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
    } else {
        await lockStdin(() => question(message + " [Enter] "));
    }
}
