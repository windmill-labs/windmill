import { escape } from "./util.ts";
import { dedent } from "../../string.ts";
import { platform } from "../../runtime.ts";
import { isWSL, lockStdin, powershell, run, which } from "../../cli.ts";
import question from "./question.ts";

function createAppleScript(message: string, defaultValue = "", password = false) {
    return dedent`
        tell application (path to frontmost application as text)
            set response to display dialog "${escape(message)}" with title "Prompt"\
                default answer "${escape(defaultValue)}"${password ? " hidden answer true" : ""}
            return text returned of response
        end
        `;
}

function createPowerShellScript(message: string, defaultValue = "", password = false) {
    return dedent`
        Add-Type -AssemblyName System.Windows.Forms
        $form = New-Object System.Windows.Forms.Form
        $form.Text = 'Prompt'
        $form.Size = New-Object System.Drawing.Size(450,175)
        $form.StartPosition = 'CenterScreen'
        $form.FormBorderStyle = 'FixedDialog'
        $form.Font = New-Object System.Drawing.Font('Aria', 10)
        $form.AutoScaleMode = 'Dpi'
        $form.MaximizeBox = $false
        $form.MinimizeBox = $false
        
        $label = New-Object System.Windows.Forms.Label
        $label.Location = New-Object System.Drawing.Point(17,20)
        $label.Size = New-Object System.Drawing.Size(400,30)
        $label.Text = "${escape(message)}"
        $form.Controls.Add($label)
        
        $textBox = New-Object System.Windows.Forms.TextBox
        $textBox.Location = New-Object System.Drawing.Point(17,50)
        $textBox.Size = New-Object System.Drawing.Size(400,30)
        $textBox.UseSystemPasswordChar = ${password ? "$true" : "$false"}
        $textBox.Text = "${escape(defaultValue)}"
        $form.Controls.Add($textBox)
        
        $cancelButton = New-Object System.Windows.Forms.Button
        $cancelButton.Location = New-Object System.Drawing.Point(232,90)
        $cancelButton.Size = New-Object System.Drawing.Size(87,27)
        $cancelButton.Text = 'Cancel'
        $cancelButton.DialogResult = [System.Windows.Forms.DialogResult]::Cancel
        $cancelButton.FlatStyle = 'System'
        $form.Controls.Add($cancelButton)
        
        $okButton = New-Object System.Windows.Forms.Button
        $okButton.Location = New-Object System.Drawing.Point(330,90)
        $okButton.Size = New-Object System.Drawing.Size(87,27)
        $okButton.Text = 'OK'
        $okButton.DialogResult = [System.Windows.Forms.DialogResult]::OK
        $okButton.FlatStyle = 'System'
        $form.Controls.Add($okButton)
        
        $form.AcceptButton = $okButton
        $form.CancelButton = $cancelButton
        $form.Add_Shown({$textBox.Select()})
        
        $result = $form.ShowDialog()
        
        if ($result -eq [System.Windows.Forms.DialogResult]::OK)
        {
            $textBox.Text
        }
        elseif ($result -eq [System.Windows.Forms.DialogResult]::Cancel)
        {
            throw 'User canceled'
        }
        `;
}

export default async function promptInTerminal(message: string, options: {
    defaultValue?: string | undefined;
    type?: "text" | "password";
    mask?: string | undefined;
    gui?: boolean;
} = {}): Promise<string | null> {
    if (options?.gui && platform() === "darwin") {
        const { code, stdout, stderr } = await run("osascript", [
            "-e",
            createAppleScript(message, options.defaultValue, options.type === "password")
        ]);

        if (code) {
            if (stderr.includes("User canceled")) {
                return null;
            } else {
                throw new Error(stderr);
            }
        } else {
            return stdout.trim();
        }
    } else if (options?.gui && (platform() === "windows" || isWSL())) {
        const { code, stdout, stderr } = await powershell(
            createPowerShellScript(message, options.defaultValue, options.type === "password")
        );

        if (code) {
            if (stderr.includes("User canceled")) {
                return null;
            } else {
                throw new Error(stderr);
            }
        } else {
            return stdout.trim();
        }
    } else if (options?.gui && (platform() === "linux" || await which("zenity"))) {
        const args = [
            "--entry",
            "--title", "Prompt",
            "--width", "450",
        ];

        if (message) {
            args.push("--text", message);
        }

        if (options.defaultValue) {
            args.push("--entry-text", options.defaultValue);
        }

        if (options.type === "password") {
            args.push("--hide-text");
        }

        const { code, stdout, stderr } = await run("zenity", args);

        if (!code) {
            return stdout.trim();
        } else if (code === 1) {
            return null;
        } else {
            throw new Error(stderr);
        }
    } else {
        return await lockStdin(() => question(message + " ", options));
    }
}
