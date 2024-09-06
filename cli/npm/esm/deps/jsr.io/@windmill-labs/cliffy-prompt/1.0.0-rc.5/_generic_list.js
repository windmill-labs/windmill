import { GenericInput, } from "./_generic_input.js";
import { bold, brightBlue, dim, stripAnsiCode, yellow } from "../../../@std/fmt/0.225.6/colors.js";
import { levenshteinDistance } from "../../../@std/text/1.0.0-rc.1/levenshtein_distance.js";
import { Figures, getFiguresByKeys } from "./_figures.js";
/** Generic list prompt representation. */
export class GenericList extends GenericInput {
    constructor() {
        super(...arguments);
        Object.defineProperty(this, "parentOptions", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: []
        });
    }
    get selectedOption() {
        return this.options.at(this.listIndex);
    }
    /**
     * Create list separator.
     *
     * @param label Separator label.
     */
    static separator(label = "------------") {
        return { name: label };
    }
    getDefaultSettings({ groupIcon = true, groupOpenIcon = groupIcon, ...options }) {
        const settings = super.getDefaultSettings(options);
        return {
            ...settings,
            listPointer: options.listPointer ?? brightBlue(Figures.POINTER),
            searchLabel: options.searchLabel ?? brightBlue(Figures.SEARCH),
            backPointer: options.backPointer ?? brightBlue(Figures.POINTER_LEFT),
            groupPointer: options.groupPointer ?? options.listPointer ??
                brightBlue(Figures.POINTER),
            groupIcon: !groupIcon
                ? false
                : typeof groupIcon === "string"
                    ? groupIcon
                    : Figures.FOLDER,
            groupOpenIcon: !groupOpenIcon
                ? false
                : typeof groupOpenIcon === "string"
                    ? groupOpenIcon
                    : Figures.FOLDER_OPEN,
            maxBreadcrumbItems: options.maxBreadcrumbItems ?? 5,
            breadcrumbSeparator: options.breadcrumbSeparator ??
                ` ${Figures.POINTER_SMALL} `,
            maxRows: options.maxRows ?? 10,
            options: this.mapOptions(options, options.options),
            keys: {
                next: options.search ? ["down"] : ["down", "d", "n", "2"],
                previous: options.search ? ["up"] : ["up", "u", "p", "8"],
                nextPage: ["pagedown", "right"],
                previousPage: ["pageup", "left"],
                open: ["right", "enter", "return"],
                back: ["left", "escape", "enter", "return"],
                ...(settings.keys ?? {}),
            },
        };
    }
    mapOption(options, option) {
        if (isOption(option)) {
            return {
                value: option.value,
                name: typeof option.name === "undefined"
                    ? options.format?.(option.value) ?? String(option.value)
                    : option.name,
                disabled: "disabled" in option && option.disabled === true,
                indentLevel: 0,
            };
        }
        else {
            return {
                value: null,
                name: option.name,
                disabled: true,
                indentLevel: 0,
            };
        }
    }
    mapOptionGroup(options, option, recursive = true) {
        return {
            name: option.name,
            disabled: !!option.disabled,
            indentLevel: 0,
            options: recursive ? this.mapOptions(options, option.options) : [],
        };
    }
    match() {
        const input = this.getCurrentInputValue().toLowerCase();
        let options = this.getCurrentOptions().slice();
        if (input.length) {
            const matches = matchOptions(input, this.getCurrentOptions());
            options = flatMatchedOptions(matches);
        }
        this.setOptions(options);
    }
    setOptions(options) {
        this.options = [...options];
        const parent = this.getParentOption();
        if (parent && this.options[0] !== parent) {
            this.options.unshift(parent);
        }
        this.listIndex = Math.max(0, Math.min(this.options.length - 1, this.listIndex));
        this.listOffset = Math.max(0, Math.min(this.options.length - this.getListHeight(), this.listOffset));
    }
    getCurrentOptions() {
        return this.getParentOption()?.options ?? this.settings.options;
    }
    getParentOption(index = -1) {
        return this.parentOptions.at(index);
    }
    submitBackButton() {
        const parentOption = this.parentOptions.pop();
        if (!parentOption) {
            return;
        }
        this.match();
        this.listIndex = this.options.indexOf(parentOption);
    }
    submitGroupOption(selectedOption) {
        this.parentOptions.push(selectedOption);
        this.match();
        this.listIndex = 0;
    }
    isBackButton(option) {
        return option === this.getParentOption();
    }
    hasParent() {
        return this.parentOptions.length > 0;
    }
    isSearching() {
        return this.getCurrentInputValue() !== "";
    }
    message() {
        let message = `${this.settings.indent}${this.settings.prefix}` +
            bold(this.settings.message) +
            this.defaults();
        if (this.settings.search) {
            const input = this.isSearchSelected() ? this.input() : dim(this.input());
            message += " " + this.settings.searchLabel + " ";
            this.cursor.x = stripAnsiCode(message).length + this.inputIndex + 1;
            message += input;
        }
        return message;
    }
    /** Render options. */
    body() {
        return this.getList() + this.getInfo();
    }
    getInfo() {
        if (!this.settings.info) {
            return "";
        }
        const selected = this.listIndex + 1;
        const hasGroups = this.options.some((option) => isOptionGroup(option));
        const groupActions = hasGroups
            ? [
                ["Open", getFiguresByKeys(this.settings.keys.open ?? [])],
                ["Back", getFiguresByKeys(this.settings.keys.back ?? [])],
            ]
            : [];
        const actions = [
            ["Next", getFiguresByKeys(this.settings.keys.next ?? [])],
            ["Previous", getFiguresByKeys(this.settings.keys.previous ?? [])],
            ...groupActions,
            ["Next Page", getFiguresByKeys(this.settings.keys.nextPage ?? [])],
            [
                "Previous Page",
                getFiguresByKeys(this.settings.keys.previousPage ?? []),
            ],
            ["Submit", getFiguresByKeys(this.settings.keys.submit ?? [])],
        ];
        return "\n" + this.settings.indent + brightBlue(Figures.INFO) +
            bold(` ${selected}/${this.options.length} `) +
            actions
                .map((cur) => `${cur[0]}: ${bold(cur[1].join(", "))}`)
                .join(", ");
    }
    /** Render options list. */
    getList() {
        const list = [];
        const height = this.getListHeight();
        for (let i = this.listOffset; i < this.listOffset + height; i++) {
            list.push(this.getListItem(this.options[i], this.listIndex === i));
        }
        if (!list.length) {
            list.push(this.settings.indent + dim("  No matches..."));
        }
        return list.join("\n");
    }
    /**
     * Render option.
     * @param option        Option.
     * @param isSelected  Set to true if option is selected.
     */
    getListItem(option, isSelected) {
        let line = this.getListItemIndent(option);
        line += this.getListItemPointer(option, isSelected);
        line += this.getListItemIcon(option);
        line += this.getListItemLabel(option, isSelected);
        return line;
    }
    getListItemIndent(option) {
        const indentLevel = this.isSearching()
            ? option.indentLevel
            : this.hasParent() && !this.isBackButton(option)
                ? 1
                : 0;
        return this.settings.indent + " ".repeat(indentLevel);
    }
    getListItemPointer(option, isSelected) {
        if (!isSelected) {
            return "  ";
        }
        if (this.isBackButton(option)) {
            return this.settings.backPointer + " ";
        }
        else if (isOptionGroup(option)) {
            return this.settings.groupPointer + " ";
        }
        return this.settings.listPointer + " ";
    }
    getListItemIcon(option) {
        if (this.isBackButton(option)) {
            return this.settings.groupOpenIcon
                ? this.settings.groupOpenIcon + " "
                : "";
        }
        else if (isOptionGroup(option)) {
            return this.settings.groupIcon ? this.settings.groupIcon + " " : "";
        }
        return "";
    }
    getListItemLabel(option, isSelected) {
        let label = option.name;
        if (this.isBackButton(option)) {
            label = this.getBreadCrumb();
            label = isSelected && !option.disabled ? label : yellow(label);
        }
        else {
            label = isSelected && !option.disabled
                ? this.highlight(label, (val) => val)
                : this.highlight(label);
        }
        if (this.isBackButton(option) || isOptionGroup(option)) {
            label = bold(label);
        }
        return label;
    }
    getBreadCrumb() {
        if (!this.parentOptions.length || !this.settings.maxBreadcrumbItems) {
            return "";
        }
        const names = this.parentOptions.map((option) => option.name);
        const breadCrumb = names.length > this.settings.maxBreadcrumbItems
            ? [names[0], "..", ...names.slice(-this.settings.maxBreadcrumbItems + 1)]
            : names;
        return breadCrumb.join(this.settings.breadcrumbSeparator);
    }
    /** Get options row height. */
    getListHeight() {
        return Math.min(this.options.length, this.settings.maxRows || this.options.length);
    }
    getListIndex(value) {
        return Math.max(0, typeof value === "undefined"
            ? this.options.findIndex((option) => !option.disabled) || 0
            : this.options.findIndex((option) => isOption(option) && option.value === value) ||
                0);
    }
    getPageOffset(index) {
        if (index === 0) {
            return 0;
        }
        const height = this.getListHeight();
        return Math.min(Math.floor(index / height) * height, this.options.length - height);
    }
    /**
     * Find option by value.
     * @param value Value of the option.
     */
    getOptionByValue(value) {
        const option = this.options.find((option) => isOption(option) && option.value === value);
        return option && isOptionGroup(option) ? undefined : option;
    }
    /** Read user input. */
    read() {
        if (!this.settings.search) {
            this.settings.tty.cursorHide();
        }
        return super.read();
    }
    selectSearch() {
        this.listIndex = -1;
    }
    isSearchSelected() {
        return this.listIndex === -1;
    }
    /**
     * Handle user input event.
     * @param event Key event.
     */
    async handleEvent(event) {
        if (this.isKey(this.settings.keys, "open", event) &&
            isOptionGroup(this.selectedOption) &&
            !this.isSearchSelected()) {
            if (this.isBackButton(this.selectedOption)) {
                this.selectNext();
            }
            else {
                this.submitGroupOption(this.selectedOption);
            }
        }
        else if (this.isKey(this.settings.keys, "back", event) &&
            (this.isBackButton(this.selectedOption) || event.name === "escape") &&
            !this.isSearchSelected()) {
            this.submitBackButton();
        }
        else if (this.isKey(this.settings.keys, "next", event)) {
            this.selectNext();
        }
        else if (this.isKey(this.settings.keys, "previous", event)) {
            this.selectPrevious();
        }
        else if (this.isKey(this.settings.keys, "nextPage", event) &&
            !this.isSearchSelected()) {
            this.selectNextPage();
        }
        else if (this.isKey(this.settings.keys, "previousPage", event) &&
            !this.isSearchSelected()) {
            this.selectPreviousPage();
        }
        else {
            await super.handleEvent(event);
        }
    }
    async submit() {
        if (this.isSearchSelected()) {
            this.selectNext();
            return;
        }
        await super.submit();
    }
    moveCursorLeft() {
        if (this.settings.search) {
            super.moveCursorLeft();
        }
    }
    moveCursorRight() {
        if (this.settings.search) {
            super.moveCursorRight();
        }
    }
    deleteChar() {
        if (this.settings.search) {
            super.deleteChar();
        }
    }
    deleteCharRight() {
        if (this.settings.search) {
            super.deleteCharRight();
            this.match();
        }
    }
    addChar(char) {
        if (this.settings.search) {
            super.addChar(char);
            this.match();
        }
    }
    /** Select previous option. */
    selectPrevious(loop = true) {
        if (this.options.length < 2 && !this.isSearchSelected()) {
            return;
        }
        if (this.listIndex > 0) {
            this.listIndex--;
            if (this.listIndex < this.listOffset) {
                this.listOffset--;
            }
            if (this.selectedOption?.disabled) {
                this.selectPrevious();
            }
        }
        else if (this.settings.search && this.listIndex === 0 &&
            this.getCurrentInputValue().length) {
            this.listIndex = -1;
        }
        else if (loop) {
            this.listIndex = this.options.length - 1;
            this.listOffset = this.options.length - this.getListHeight();
            if (this.selectedOption?.disabled) {
                this.selectPrevious();
            }
        }
    }
    /** Select next option. */
    selectNext(loop = true) {
        if (this.options.length < 2 && !this.isSearchSelected()) {
            return;
        }
        if (this.listIndex < this.options.length - 1) {
            this.listIndex++;
            if (this.listIndex >= this.listOffset + this.getListHeight()) {
                this.listOffset++;
            }
            if (this.selectedOption?.disabled) {
                this.selectNext();
            }
        }
        else if (this.settings.search && this.listIndex === this.options.length - 1 &&
            this.getCurrentInputValue().length) {
            this.listIndex = -1;
        }
        else if (loop) {
            this.listIndex = this.listOffset = 0;
            if (this.selectedOption?.disabled) {
                this.selectNext();
            }
        }
    }
    /** Select previous page. */
    selectPreviousPage() {
        if (this.options?.length) {
            const height = this.getListHeight();
            if (this.listOffset >= height) {
                this.listIndex -= height;
                this.listOffset -= height;
            }
            else if (this.listOffset > 0) {
                this.listIndex -= this.listOffset;
                this.listOffset = 0;
            }
            else {
                this.listIndex = 0;
            }
            if (this.selectedOption?.disabled) {
                this.selectPrevious(false);
            }
            if (this.selectedOption?.disabled) {
                this.selectNext(false);
            }
        }
    }
    /** Select next page. */
    selectNextPage() {
        if (this.options?.length) {
            const height = this.getListHeight();
            if (this.listOffset + height + height < this.options.length) {
                this.listIndex += height;
                this.listOffset += height;
            }
            else if (this.listOffset + height < this.options.length) {
                const offset = this.options.length - height;
                this.listIndex += offset - this.listOffset;
                this.listOffset = offset;
            }
            else {
                this.listIndex = this.options.length - 1;
            }
            if (this.selectedOption?.disabled) {
                this.selectNext(false);
            }
            if (this.selectedOption?.disabled) {
                this.selectPrevious(false);
            }
        }
    }
}
export function isOption(option) {
    return !!option && typeof option === "object" && "value" in option;
}
export function isOptionGroup(option) {
    return option !== null && typeof option === "object" && "options" in option &&
        Array.isArray(option.options);
}
function matchOptions(searchInput, options) {
    const matched = [];
    for (const option of options) {
        if (isOptionGroup(option)) {
            const children = matchOptions(searchInput, option.options)
                .sort(sortByDistance);
            if (children.length) {
                matched.push({
                    option,
                    distance: Math.min(...children.map((item) => item.distance)),
                    children,
                });
                continue;
            }
        }
        if (matchOption(searchInput, option)) {
            matched.push({
                option,
                distance: levenshteinDistance(option.name, searchInput),
                children: [],
            });
        }
    }
    return matched.sort(sortByDistance);
    function sortByDistance(a, b) {
        return a.distance - b.distance;
    }
}
function matchOption(inputString, option) {
    return matchInput(inputString, option.name) || (isOption(option) &&
        option.name !== option.value &&
        matchInput(inputString, String(option.value)));
}
function matchInput(inputString, value) {
    return stripAnsiCode(value)
        .toLowerCase()
        .includes(inputString);
}
function flatMatchedOptions(matches, indentLevel = 0, result = []) {
    for (const { option, children } of matches) {
        option.indentLevel = indentLevel;
        result.push(option);
        flatMatchedOptions(children, indentLevel + 1, result);
    }
    return result;
}
