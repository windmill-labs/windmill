import { extname } from '../../../path.js';
import { lines } from '../../../string.js';
import { run } from '../../../cli.js';
import { getExtensions } from '../../../filetype.js';

function htmlAcceptToFileFilters(accept) {
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
            return group.map(type => getExtensions(type).map(t => `*${t}`))
                .flat()
                .join(" ");
        }
        else if (group === "*/*") {
            return "All Files | *";
        }
        else {
            const patterns = getExtensions(group).map(t => `*${t}`).join(" ");
            if (!patterns) {
                return undefined;
            }
            else if (group === "video/*") {
                return "Video Files | " + patterns;
            }
            else if (group === "audio/*") {
                return "Audio Files | " + patterns;
            }
            else if (group === "image/*") {
                return "Image Files | " + patterns;
            }
            else if (group === "text/*") {
                return "Text Files | " + patterns;
            }
            else {
                return patterns;
            }
        }
    }).filter(Boolean);
}
async function linuxPickFile(title = "", options = {}) {
    const { type, forSave, defaultName } = options;
    const args = [
        "--file-selection",
    ];
    if (title) {
        args.push("--title", title);
    }
    if (type) {
        htmlAcceptToFileFilters(type).forEach(filter => {
            args.push("--file-filter", filter);
        });
    }
    if (forSave) {
        args.push("--save", "--confirm-overwrite");
        if (defaultName) {
            args.push("--filename", defaultName);
            if (!type) {
                const ext = extname(defaultName);
                if (ext) {
                    htmlAcceptToFileFilters(ext).forEach(filter => {
                        args.push("--file-filter", filter);
                    });
                }
            }
        }
    }
    const { code, stdout, stderr } = await run("zenity", args);
    if (!code) {
        const path = stdout.trim();
        return path || null;
    }
    else if (code === 1) {
        return null;
    }
    else {
        throw new Error(stderr.trim());
    }
}
async function linuxPickFiles(title = "", type = "") {
    const args = [
        "--file-selection",
        "--multiple",
        "--separator", "\n",
    ];
    if (title) {
        args.push("--title", title);
    }
    if (type) {
        htmlAcceptToFileFilters(type).forEach(filter => {
            args.push("--file-filter", filter);
        });
    }
    const { code, stdout, stderr } = await run("zenity", args);
    if (!code) {
        const output = stdout.trim();
        return output ? lines(stdout.trim()) : [];
    }
    else if (code === 1) {
        return [];
    }
    else {
        throw new Error(stderr.trim());
    }
}
async function linuxPickFolder(title = "") {
    const args = [
        "--file-selection",
        "--directory",
    ];
    if (title) {
        args.push("--title", title);
    }
    const { code, stdout, stderr } = await run("zenity", args);
    if (!code) {
        const dir = stdout.trim();
        return dir || null;
    }
    else if (code === 1) {
        return null;
    }
    else {
        throw new Error(stderr.trim());
    }
}

export { linuxPickFile, linuxPickFiles, linuxPickFolder };
//# sourceMappingURL=linux.js.map
