import { closestString } from "../../../@std/text/1.0.0-rc.1/closest_string.js";
/** Convert param case string to camel case. */
export function paramCaseToCamelCase(str) {
    return str.replace(/-([a-z])/g, (g) => g[1].toUpperCase());
}
/**
 * Find option by flag, name or alias.
 *
 * @param flags Source options array.
 * @param name  Name of the option.
 */
export function getOption(flags, name) {
    while (name[0] === "-") {
        name = name.slice(1);
    }
    for (const flag of flags) {
        if (isOption(flag, name)) {
            return flag;
        }
    }
    return;
}
export function didYouMeanOption(option, options) {
    const optionNames = options
        .map((option) => [option.name, ...(option.aliases ?? [])])
        .flat()
        .map((option) => getFlag(option));
    return didYouMean(" Did you mean option", getFlag(option), optionNames);
}
export function didYouMeanType(type, types) {
    return didYouMean(" Did you mean type", type, types);
}
export function didYouMean(message, type, types) {
    const match = types.length
        ? closestString(type, types)
        : undefined;
    return match ? `${message} "${match}"?` : "";
}
export function getFlag(name) {
    if (name.startsWith("-")) {
        return name;
    }
    if (name.length > 1) {
        return `--${name}`;
    }
    return `-${name}`;
}
/**
 * Check if option has name or alias.
 *
 * @param option    The option to check.
 * @param name      The option name or alias.
 */
function isOption(option, name) {
    return option.name === name ||
        (option.aliases && option.aliases.indexOf(name) !== -1);
}
export function matchWildCardOptions(name, flags) {
    for (const option of flags) {
        if (option.name.indexOf("*") === -1) {
            continue;
        }
        let matched = matchWildCardOption(name, option);
        if (matched) {
            matched = { ...matched, name };
            flags.push(matched);
            return matched;
        }
    }
}
function matchWildCardOption(name, option) {
    const parts = option.name.split(".");
    const parts2 = name.split(".");
    if (parts.length !== parts2.length) {
        return false;
    }
    const count = Math.max(parts.length, parts2.length);
    for (let i = 0; i < count; i++) {
        if (parts[i] !== parts2[i] && parts[i] !== "*") {
            return false;
        }
    }
    return option;
}
export function getDefaultValue(option) {
    return typeof option.default === "function"
        ? option.default()
        : option.default;
}
