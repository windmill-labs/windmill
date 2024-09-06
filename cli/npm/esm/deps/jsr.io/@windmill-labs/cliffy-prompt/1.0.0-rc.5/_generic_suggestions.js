var __classPrivateFieldGet = (this && this.__classPrivateFieldGet) || function (receiver, state, kind, f) {
    if (kind === "a" && !f) throw new TypeError("Private accessor was defined without a getter");
    if (typeof state === "function" ? receiver !== state || !f : !state.has(receiver)) throw new TypeError("Cannot read private member from an object whose class did not declare it");
    return kind === "m" ? f : kind === "a" ? f.call(receiver) : f ? f.value : state.get(receiver);
};
var __classPrivateFieldSet = (this && this.__classPrivateFieldSet) || function (receiver, state, value, kind, f) {
    if (kind === "m") throw new TypeError("Private method is not writable");
    if (kind === "a" && !f) throw new TypeError("Private accessor was defined without a setter");
    if (typeof state === "function" ? receiver !== state || !f : !state.has(receiver)) throw new TypeError("Cannot write private member to an object whose class did not declare it");
    return (kind === "a" ? f.call(receiver, value) : f ? f.value = value : state.set(receiver, value)), value;
};
var _GenericSuggestions_instances, _GenericSuggestions_envPermissions, _GenericSuggestions_hasReadPermissions, _GenericSuggestions_isFileModeEnabled, _GenericSuggestions_completeValue, _GenericSuggestions_expandInputValue, _GenericSuggestions_hasEnvPermissions;
import * as dntShim from "../../../../../_dnt.shims.js";
import { bold, brightBlue, dim, stripAnsiCode, underline, } from "../../../@std/fmt/0.225.6/colors.js";
import { dirname, join, normalize } from "../../../@std/path/1.0.0-rc.2/mod.js";
import { levenshteinDistance } from "../../../@std/text/1.0.0-rc.1/levenshtein_distance.js";
import { Figures, getFiguresByKeys } from "./_figures.js";
import { GenericInput, } from "./_generic_input.js";
import { getOs } from "../../cliffy-internal/1.0.0-rc.5/runtime/get_os.js";
import { isDirectory } from "../../cliffy-internal/1.0.0-rc.5/runtime/is_directory.js";
import { readDir } from "../../cliffy-internal/1.0.0-rc.5/runtime/read_dir.js";
import { stat } from "../../cliffy-internal/1.0.0-rc.5/runtime/stat.js";
const sep = getOs() === "windows" ? "\\" : "/";
/** Generic input prompt representation. */
export class GenericSuggestions extends GenericInput {
    constructor() {
        super(...arguments);
        _GenericSuggestions_instances.add(this);
        Object.defineProperty(this, "suggestionsIndex", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: -1
        });
        Object.defineProperty(this, "suggestionsOffset", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: 0
        });
        Object.defineProperty(this, "suggestions", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: []
        });
        _GenericSuggestions_envPermissions.set(this, {});
        _GenericSuggestions_hasReadPermissions.set(this, void 0);
    }
    getDefaultSettings(options) {
        const settings = super.getDefaultSettings(options);
        return {
            ...settings,
            listPointer: options.listPointer ?? brightBlue(Figures.POINTER),
            maxRows: options.maxRows ?? 8,
            keys: {
                complete: ["tab"],
                next: ["up"],
                previous: ["down"],
                nextPage: ["pageup"],
                previousPage: ["pagedown"],
                ...(settings.keys ?? {}),
            },
        };
    }
    get localStorage() {
        // Keep support for deno < 1.10.
        if (this.settings.id && "localStorage" in dntShim.dntGlobalThis) {
            try {
                // deno-lint-ignore no-explicit-any
                return dntShim.dntGlobalThis.localStorage;
            }
            catch (_) {
                // Ignore error if --location is not set.
            }
        }
        return null;
    }
    loadSuggestions() {
        if (this.settings.id) {
            const json = this.localStorage?.getItem(this.settings.id);
            const suggestions = json ? JSON.parse(json) : [];
            if (!Array.isArray(suggestions)) {
                return [];
            }
            return suggestions;
        }
        return [];
    }
    saveSuggestions(...suggestions) {
        if (this.settings.id) {
            this.localStorage?.setItem(this.settings.id, JSON.stringify([
                ...suggestions,
                ...this.loadSuggestions(),
            ].filter(uniqueSuggestions)));
        }
    }
    async render() {
        if (this.settings.files && __classPrivateFieldGet(this, _GenericSuggestions_hasReadPermissions, "f") === undefined) {
            // deno-lint-ignore no-explicit-any
            const status = await dntShim.dntGlobalThis.Deno?.permissions.request({
                name: "read",
            });
            // disable path completion if read permissions are denied.
            __classPrivateFieldSet(this, _GenericSuggestions_hasReadPermissions, !status || status.state === "granted", "f");
        }
        if (__classPrivateFieldGet(this, _GenericSuggestions_instances, "m", _GenericSuggestions_isFileModeEnabled).call(this)) {
            await __classPrivateFieldGet(this, _GenericSuggestions_instances, "m", _GenericSuggestions_expandInputValue).call(this, this.inputValue);
        }
        await this.match();
        return super.render();
    }
    async match() {
        this.suggestions = await this.getSuggestions();
        this.suggestionsIndex = Math.max(this.getCurrentInputValue().trim().length === 0 ? -1 : 0, Math.min(this.suggestions.length - 1, this.suggestionsIndex));
        this.suggestionsOffset = Math.max(0, Math.min(this.suggestions.length - this.getListHeight(), this.suggestionsOffset));
    }
    input() {
        return super.input() + dim(this.getSuggestion());
    }
    getSuggestion() {
        return this.suggestions[this.suggestionsIndex]?.toString()
            .substr(this.getCurrentInputValue().length) ?? "";
    }
    async getUserSuggestions(input) {
        return typeof this.settings.suggestions === "function"
            ? await this.settings.suggestions(input)
            : this.settings.suggestions ?? [];
    }
    async getFileSuggestions(input) {
        if (!__classPrivateFieldGet(this, _GenericSuggestions_instances, "m", _GenericSuggestions_isFileModeEnabled).call(this)) {
            return [];
        }
        const path = await stat(input)
            .then((file) => file.isDirectory ? input : dirname(input))
            .catch(() => dirname(input));
        return await listDir(path, this.settings.files);
    }
    async getSuggestions() {
        const input = this.getCurrentInputValue();
        const suggestions = [
            ...this.loadSuggestions(),
            ...await this.getUserSuggestions(input),
            ...await this.getFileSuggestions(input),
        ].filter(uniqueSuggestions);
        if (!input.length) {
            return suggestions;
        }
        return suggestions
            .filter((value) => stripAnsiCode(value.toString())
            .toLowerCase()
            .startsWith(input.toLowerCase()))
            .sort((a, b) => levenshteinDistance((a || a).toString(), input) -
            levenshteinDistance((b || b).toString(), input));
    }
    body() {
        return this.getList() + this.getInfo();
    }
    getInfo() {
        if (!this.settings.info) {
            return "";
        }
        const selected = this.suggestionsIndex + 1;
        const matched = this.suggestions.length;
        const actions = [];
        if (this.suggestions.length) {
            if (this.settings.list) {
                actions.push(["Next", getFiguresByKeys(this.settings.keys?.next ?? [])], ["Previous", getFiguresByKeys(this.settings.keys?.previous ?? [])], ["Next Page", getFiguresByKeys(this.settings.keys?.nextPage ?? [])], [
                    "Previous Page",
                    getFiguresByKeys(this.settings.keys?.previousPage ?? []),
                ]);
            }
            else {
                actions.push(["Next", getFiguresByKeys(this.settings.keys?.next ?? [])], ["Previous", getFiguresByKeys(this.settings.keys?.previous ?? [])]);
            }
            actions.push(["Complete", getFiguresByKeys(this.settings.keys?.complete ?? [])]);
        }
        actions.push(["Submit", getFiguresByKeys(this.settings.keys?.submit ?? [])]);
        let info = this.settings.indent;
        if (this.suggestions.length) {
            info += brightBlue(Figures.INFO) + bold(` ${selected}/${matched} `);
        }
        info += actions
            .map((cur) => `${cur[0]}: ${bold(cur[1].join(" "))}`)
            .join(", ");
        return info;
    }
    getList() {
        if (!this.suggestions.length || !this.settings.list) {
            return "";
        }
        const list = [];
        const height = this.getListHeight();
        for (let i = this.suggestionsOffset; i < this.suggestionsOffset + height; i++) {
            list.push(this.getListItem(this.suggestions[i], this.suggestionsIndex === i));
        }
        if (list.length && this.settings.info) {
            list.push("");
        }
        return list.join("\n");
    }
    /**
     * Render option.
     * @param value        Option.
     * @param isSelected  Set to true if option is selected.
     */
    getListItem(value, isSelected) {
        let line = this.settings.indent ?? "";
        line += isSelected ? `${this.settings.listPointer} ` : "  ";
        if (isSelected) {
            line += underline(this.highlight(value));
        }
        else {
            line += this.highlight(value);
        }
        return line;
    }
    /** Get suggestions row height. */
    getListHeight(suggestions = this.suggestions) {
        return Math.min(suggestions.length, this.settings.maxRows || suggestions.length);
    }
    /**
     * Handle user input event.
     * @param event Key event.
     */
    async handleEvent(event) {
        switch (true) {
            case this.isKey(this.settings.keys, "next", event):
                if (this.settings.list) {
                    this.selectPreviousSuggestion();
                }
                else {
                    this.selectNextSuggestion();
                }
                break;
            case this.isKey(this.settings.keys, "previous", event):
                if (this.settings.list) {
                    this.selectNextSuggestion();
                }
                else {
                    this.selectPreviousSuggestion();
                }
                break;
            case this.isKey(this.settings.keys, "nextPage", event):
                if (this.settings.list) {
                    this.selectPreviousSuggestionsPage();
                }
                else {
                    this.selectNextSuggestionsPage();
                }
                break;
            case this.isKey(this.settings.keys, "previousPage", event):
                if (this.settings.list) {
                    this.selectNextSuggestionsPage();
                }
                else {
                    this.selectPreviousSuggestionsPage();
                }
                break;
            case this.isKey(this.settings.keys, "complete", event):
                await __classPrivateFieldGet(this, _GenericSuggestions_instances, "m", _GenericSuggestions_completeValue).call(this);
                break;
            case this.isKey(this.settings.keys, "moveCursorRight", event):
                if (this.inputIndex < this.inputValue.length) {
                    this.moveCursorRight();
                }
                else {
                    await __classPrivateFieldGet(this, _GenericSuggestions_instances, "m", _GenericSuggestions_completeValue).call(this);
                }
                break;
            default:
                await super.handleEvent(event);
        }
    }
    /** Delete char right. */
    deleteCharRight() {
        if (this.inputIndex < this.inputValue.length) {
            super.deleteCharRight();
            if (!this.getCurrentInputValue().length) {
                this.suggestionsIndex = -1;
                this.suggestionsOffset = 0;
            }
        }
    }
    setInputValue(inputValue) {
        this.inputValue = inputValue;
        this.inputIndex = this.inputValue.length;
        this.suggestionsIndex = 0;
        this.suggestionsOffset = 0;
    }
    async complete() {
        let input = this.getCurrentInputValue();
        const suggestion = this
            .suggestions[this.suggestionsIndex]?.toString();
        if (this.settings.complete) {
            input = await this.settings.complete(input, suggestion);
        }
        else if (__classPrivateFieldGet(this, _GenericSuggestions_instances, "m", _GenericSuggestions_isFileModeEnabled).call(this) &&
            input.at(-1) !== sep &&
            await isDirectory(input) &&
            (this.getCurrentInputValue().at(-1) !== "." ||
                this.getCurrentInputValue().endsWith(".."))) {
            input += sep;
        }
        else if (suggestion) {
            input = suggestion;
        }
        return __classPrivateFieldGet(this, _GenericSuggestions_instances, "m", _GenericSuggestions_isFileModeEnabled).call(this) ? normalize(input) : input;
    }
    /** Select previous suggestion. */
    selectPreviousSuggestion() {
        if (this.suggestions.length) {
            if (this.suggestionsIndex > -1) {
                this.suggestionsIndex--;
                if (this.suggestionsIndex < this.suggestionsOffset) {
                    this.suggestionsOffset--;
                }
            }
        }
    }
    /** Select next suggestion. */
    selectNextSuggestion() {
        if (this.suggestions.length) {
            if (this.suggestionsIndex < this.suggestions.length - 1) {
                this.suggestionsIndex++;
                if (this.suggestionsIndex >=
                    this.suggestionsOffset + this.getListHeight()) {
                    this.suggestionsOffset++;
                }
            }
        }
    }
    /** Select previous suggestions page. */
    selectPreviousSuggestionsPage() {
        if (this.suggestions.length) {
            const height = this.getListHeight();
            if (this.suggestionsOffset >= height) {
                this.suggestionsIndex -= height;
                this.suggestionsOffset -= height;
            }
            else if (this.suggestionsOffset > 0) {
                this.suggestionsIndex -= this.suggestionsOffset;
                this.suggestionsOffset = 0;
            }
        }
    }
    /** Select next suggestions page. */
    selectNextSuggestionsPage() {
        if (this.suggestions.length) {
            const height = this.getListHeight();
            if (this.suggestionsOffset + height + height < this.suggestions.length) {
                this.suggestionsIndex += height;
                this.suggestionsOffset += height;
            }
            else if (this.suggestionsOffset + height < this.suggestions.length) {
                const offset = this.suggestions.length - height;
                this.suggestionsIndex += offset - this.suggestionsOffset;
                this.suggestionsOffset = offset;
            }
        }
    }
}
_GenericSuggestions_envPermissions = new WeakMap(), _GenericSuggestions_hasReadPermissions = new WeakMap(), _GenericSuggestions_instances = new WeakSet(), _GenericSuggestions_isFileModeEnabled = function _GenericSuggestions_isFileModeEnabled() {
    return !!this.settings.files && __classPrivateFieldGet(this, _GenericSuggestions_hasReadPermissions, "f") === true;
}, _GenericSuggestions_completeValue = async function _GenericSuggestions_completeValue() {
    const inputValue = await this.complete();
    this.setInputValue(inputValue);
}, _GenericSuggestions_expandInputValue = async function _GenericSuggestions_expandInputValue(path) {
    if (!path.startsWith("~")) {
        return;
    }
    const envVar = getHomeDirEnvVar();
    const hasEnvPermissions = await __classPrivateFieldGet(this, _GenericSuggestions_instances, "m", _GenericSuggestions_hasEnvPermissions).call(this, envVar);
    if (!hasEnvPermissions) {
        return;
    }
    const homeDir = getHomeDir();
    if (homeDir) {
        path = path.replace("~", homeDir);
        this.setInputValue(path);
    }
}, _GenericSuggestions_hasEnvPermissions = async function _GenericSuggestions_hasEnvPermissions(variable) {
    if (__classPrivateFieldGet(this, _GenericSuggestions_envPermissions, "f")[variable]) {
        return __classPrivateFieldGet(this, _GenericSuggestions_envPermissions, "f")[variable];
    }
    const desc = {
        name: "env",
        variable,
    };
    const currentStatus = await dntShim.Deno.permissions.query(desc);
    __classPrivateFieldGet(this, _GenericSuggestions_envPermissions, "f")[variable] = currentStatus.state === "granted";
    if (!__classPrivateFieldGet(this, _GenericSuggestions_envPermissions, "f")[variable]) {
        this.clear();
        const newStatus = await dntShim.Deno.permissions.request(desc);
        __classPrivateFieldGet(this, _GenericSuggestions_envPermissions, "f")[variable] = newStatus.state === "granted";
    }
    return __classPrivateFieldGet(this, _GenericSuggestions_envPermissions, "f")[variable];
};
function uniqueSuggestions(value, index, self) {
    return typeof value !== "undefined" && value !== "" &&
        self.indexOf(value) === index;
}
async function listDir(path, mode) {
    const fileNames = [];
    for (const file of await readDir(path)) {
        if (mode === true && (file.name.startsWith(".") || file.name.endsWith("~"))) {
            continue;
        }
        const filePath = join(path, file.name);
        if (mode instanceof RegExp && !mode.test(filePath)) {
            continue;
        }
        fileNames.push(filePath);
    }
    return fileNames.sort(function (a, b) {
        return a.toLowerCase().localeCompare(b.toLowerCase());
    });
}
function getHomeDirEnvVar() {
    return dntShim.Deno.build.os === "windows" ? "USERPROFILE" : "HOME";
}
function getHomeDir() {
    return dntShim.Deno.env.get(getHomeDirEnvVar());
}
