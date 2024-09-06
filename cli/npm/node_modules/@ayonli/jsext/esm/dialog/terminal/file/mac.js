import { lines, dedent } from '../../../string.js';
import { run } from '../../../cli.js';
import { getUTI } from '../../../filetype.js';
import { escape } from '../util.js';

function htmlAcceptToAppleType(accept) {
    return accept.split(/\s*,\s*/).map(getUTI).filter(Boolean).map(t => `"${t}"`).join(", ");
}
function createAppleScript(mode, title = "", options = {}) {
    const { type, forSave, defaultName } = options;
    if (mode === "file") {
        if (forSave) {
            return dedent `
                tell application (path to frontmost application as text)
                    set myFile to choose file name${title ? ` with prompt "${escape(title)}"` : ""}\
                        ${defaultName ? ` default name "${escape(defaultName)}"` : ""}
                    POSIX path of myFile
                end
                `;
        }
        else {
            const _type = type ? htmlAcceptToAppleType(type) : "";
            return dedent `
                tell application (path to frontmost application as text)
                    set myFile to choose file${title ? ` with prompt "${escape(title)}"` : ""}\
                        ${_type ? ` of type {${_type}}` : ""} invisibles false
                    POSIX path of myFile
                end
                `;
        }
    }
    else if (mode === "files") {
        const _type = type ? htmlAcceptToAppleType(type) : "";
        return dedent `
            tell application (path to frontmost application as text)
                set myFiles to choose file${title ? ` with prompt "${escape(title)}"` : ""}\
                    ${_type ? ` of type {${_type}}` : ""} invisibles false\
                    multiple selections allowed true
                set theList to {}
                repeat with aItem in myFiles
                    set end of theList to POSIX path of aItem
                end repeat
                set my text item delimiters to "\\n"
                return theList as text
            end
            `;
    }
    else {
        return dedent `
            tell application (path to frontmost application as text)
                set myFolder to choose folder${title ? ` with prompt "${escape(title)}"` : ""}\
                    invisibles false
                POSIX path of myFolder
            end
            `;
    }
}
async function macPickFile(title = "", options = {}) {
    const { code, stdout, stderr } = await run("osascript", [
        "-e",
        createAppleScript("file", title, options)
    ]);
    if (!code) {
        const path = stdout.trim();
        return path || null;
    }
    else {
        if (stderr.includes("User canceled")) {
            return null;
        }
        else {
            throw new Error(stderr.trim());
        }
    }
}
async function macPickFiles(title = "", type = "") {
    const { code, stdout, stderr } = await run("osascript", [
        "-e",
        createAppleScript("files", title, { type })
    ]);
    if (!code) {
        const output = stdout.trim();
        return output ? lines(stdout.trim()) : [];
    }
    else {
        if (stderr.includes("User canceled")) {
            return [];
        }
        else {
            throw new Error(stderr.trim());
        }
    }
}
async function macPickFolder(title = "") {
    const { code, stdout, stderr } = await run("osascript", [
        "-e",
        createAppleScript("folder", title)
    ]);
    if (!code) {
        const dir = stdout.trim();
        return dir || null;
    }
    else {
        if (stderr.includes("User canceled")) {
            return null;
        }
        else {
            throw new Error(stderr.trim());
        }
    }
}

export { macPickFile, macPickFiles, macPickFolder };
//# sourceMappingURL=mac.js.map
