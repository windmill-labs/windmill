import { extname } from '../../../path.js';
import { lines, dedent } from '../../../string.js';
import { powershell } from '../../../cli.js';
import { getExtensions } from '../../../filetype.js';
import { escape } from '../util.js';
import { isWSL } from '../../../cli/common.js';

function htmlAcceptToFileFilter(accept) {
    const groups = [];
    for (const type of accept.split(/\s*,\s*/)) {
        if (type.endsWith("/*")) {
            groups.push(type);
        }
        else {
            const group = groups[groups.length - 1];
            if (!group || typeof group === "string") {
                groups.push([type]);
            }
            else {
                group.push(type);
            }
        }
    }
    return groups.map(group => {
        if (Array.isArray(group)) {
            const patterns = group.map(type => getExtensions(type).map(t => `*${t}`))
                .flat()
                .join(";");
            return patterns + "|" + patterns;
        }
        else if (group === "*/*") {
            return "All Files|*";
        }
        else {
            const patterns = getExtensions(group).map(t => `*${t}`).join(";");
            if (!patterns) {
                return undefined;
            }
            else if (group === "video/*") {
                return "Video Files|" + patterns;
            }
            else if (group === "audio/*") {
                return "Audio Files|" + patterns;
            }
            else if (group === "image/*") {
                return "Image Files|" + patterns;
            }
            else if (group === "text/*") {
                return "Text Files|" + patterns;
            }
            else {
                return patterns;
            }
        }
    }).filter(Boolean).join("|");
}
function createPowerShellScript(mode, title = "", options = {}) {
    const { type, forSave, defaultName } = options;
    if (mode === "file") {
        if (forSave) {
            let filter = type ? htmlAcceptToFileFilter(type) : "";
            if (!filter && defaultName) {
                const ext = extname(defaultName);
                ext && (filter = htmlAcceptToFileFilter(ext));
            }
            return dedent `
                Add-Type -AssemblyName System.Windows.Forms
                $saveFileDialog = [System.Windows.Forms.SaveFileDialog]::new()
                $saveFileDialog.Title = "${escape(title)}"
                $saveFileDialog.FileName = "${defaultName ? escape(defaultName) : ""}"
                $saveFileDialog.Filter = "${filter}"
                if ($saveFileDialog.ShowDialog() -eq 'OK') {
                    $saveFileDialog.FileName
                }
                `;
        }
        else {
            const filter = type ? htmlAcceptToFileFilter(type) : "";
            return dedent `
                Add-Type -AssemblyName System.Windows.Forms
                $openFileDialog = [System.Windows.Forms.OpenFileDialog]::new()
                $openFileDialog.Title = "${escape(title)}"
                $openFileDialog.Filter = "${filter}"
                $openFileDialog.Multiselect = $false
                $openFileDialog.ShowDialog() | Out-Null
                $openFileDialog.FileName
                `;
        }
    }
    else if (mode === "files") {
        const filter = type ? htmlAcceptToFileFilter(type) : "";
        return dedent `
            Add-Type -AssemblyName System.Windows.Forms
            $openFileDialog = [System.Windows.Forms.OpenFileDialog]::new()
            $openFileDialog.Title = "${escape(title)}"
            $openFileDialog.Filter = "${filter}"
            $openFileDialog.Multiselect = $true
            $openFileDialog.ShowDialog() | Out-Null
            $openFileDialog.FileNames -join "\n"
            `;
    }
    else {
        return dedent `
            Add-Type -AssemblyName System.Windows.Forms
            $folderBrowserDialog = [System.Windows.Forms.FolderBrowserDialog]::new()
            $folderBrowserDialog.Description = "${escape(title)}"
            $folderBrowserDialog.ShowDialog() | Out-Null
            $folderBrowserDialog.SelectedPath
            `;
    }
}
function refinePath(path) {
    if (isWSL()) {
        return "/mnt/"
            + path.replace(/\\/g, "/").replace(/^([a-z]):/i, (_, $1) => $1.toLowerCase());
    }
    return path;
}
async function windowsPickFile(title = "", options = {}) {
    const { code, stdout, stderr } = await powershell(createPowerShellScript("file", title, options));
    if (!code) {
        const path = stdout.trim();
        return path ? refinePath(path) : null;
    }
    else {
        throw new Error(stderr);
    }
}
async function windowsPickFiles(title = "", type = "") {
    const { code, stdout, stderr } = await powershell(createPowerShellScript("files", title, { type }));
    if (!code) {
        const output = stdout.trim();
        return output ? lines(stdout.trim()).map(refinePath) : [];
    }
    else {
        throw new Error(stderr);
    }
}
async function windowsPickFolder(title = "") {
    const { code, stdout, stderr } = await powershell(createPowerShellScript("folder", title));
    if (!code) {
        const dir = stdout.trim();
        return dir ? refinePath(dir) : null;
    }
    else {
        throw new Error(stderr);
    }
}

export { windowsPickFile, windowsPickFiles, windowsPickFolder };
//# sourceMappingURL=windows.js.map
