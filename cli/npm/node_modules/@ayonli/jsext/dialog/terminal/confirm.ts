import { escape } from "./util.ts";
import { dedent } from "../../string.ts";
import { platform } from "../../runtime.ts";
import { isWSL, lockStdin, powershell, run, which } from "../../cli.ts";
import question from "./question.ts";

function createAppleScript(message: string) {
    return dedent`
        tell application (path to frontmost application as text)
            display dialog "${escape(message)}" with title "Confirm"\
                buttons {"Cancel", "OK"} default button "OK"
        end
        `;
}

function createPowerShellScript(message: string) {
    return dedent`
        Add-Type -AssemblyName PresentationFramework
        [System.Windows.MessageBox]::Show("${escape(message)}", "Confirm", "YesNo")
        `;
}

export default async function confirmInTerminal(message: string, options: {
    gui?: boolean;
} = {}): Promise<boolean> {
    if (options?.gui && platform() === "darwin") {
        const { code, stderr } = await run("osascript", [
            "-e",
            createAppleScript(message)
        ]);

        if (code) {
            if (stderr.includes("User canceled")) {
                return false;
            } else {
                throw new Error(stderr);
            }
        } else {
            return true;
        }
    } else if (options?.gui && (platform() === "windows" || isWSL())) {
        const { code, stdout, stderr } = await powershell(
            createPowerShellScript(message)
        );

        if (code) {
            throw new Error(stderr);
        } else {
            return stdout.trim() === "Yes" ? true : false;
        }
    } else if (options?.gui && (platform() === "linux" || await which("zenity"))) {
        const args = [
            "--question",
            "--title", "Confirm",
            "--width", "365",
        ];

        if (message) {
            args.push("--text", message);
        }

        const { code, stderr } = await run("zenity", args);

        if (!code) {
            return true;
        } else if (code === 1) {
            return false;
        } else {
            throw new Error(stderr);
        }
    } else {
        const answer = await lockStdin(() => question(message + " [Y/n] "));
        const ok = answer?.toLowerCase().trim();
        return ok === "" || ok === "y" || ok === "yes";
    }
}
