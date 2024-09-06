import { getDefaultValue, getOption, paramCaseToCamelCase } from "./_utils.js";
import { ConflictingOptionError, DependingOptionError, MissingOptionValueError, MissingRequiredOptionError, OptionNotCombinableError, UnknownOptionError, } from "./_errors.js";
/**
 * Flags post validation. Validations that are not already done by the parser.
 *
 * @param ctx     Parse context.
 * @param opts    Parse options.
 * @param options Option name mappings: propertyName -> option
 */
export function validateFlags(ctx, opts, options = new Map()) {
    if (!opts.flags) {
        return;
    }
    setDefaultValues(ctx, opts);
    const optionNames = Object.keys(ctx.flags);
    if (!optionNames.length && opts.allowEmpty) {
        return;
    }
    if (ctx.standalone) {
        validateStandaloneOption(ctx, options, optionNames);
        return;
    }
    for (const [name, option] of options) {
        validateUnknownOption(option, opts);
        validateConflictingOptions(ctx, option);
        validateDependingOptions(ctx, option);
        validateRequiredValues(ctx, option, name);
    }
    validateRequiredOptions(ctx, options, opts);
}
function validateUnknownOption(option, opts) {
    if (!getOption(opts.flags ?? [], option.name)) {
        throw new UnknownOptionError(option.name, opts.flags ?? []);
    }
}
/**
 * Adds all default values to ctx.flags and returns a boolean object map with
 * only the default option names `{ [OptionName: string]: boolean }`.
 */
function setDefaultValues(ctx, opts) {
    if (!opts.flags?.length) {
        return;
    }
    // Set default values
    for (const option of opts.flags) {
        let name;
        let defaultValue = undefined;
        // if --no-[flag] is present set --[flag] default value to true
        if (option.name.startsWith("no-")) {
            const propName = option.name.replace(/^no-/, "");
            if (typeof ctx.flags[propName] !== "undefined") {
                continue;
            }
            const positiveOption = getOption(opts.flags, propName);
            if (positiveOption) {
                continue;
            }
            name = paramCaseToCamelCase(propName);
            defaultValue = true;
        }
        if (!name) {
            name = paramCaseToCamelCase(option.name);
        }
        const hasDefaultValue = (!opts.ignoreDefaults ||
            typeof opts.ignoreDefaults[name] === "undefined") &&
            typeof ctx.flags[name] === "undefined" && (typeof option.default !== "undefined" ||
            typeof defaultValue !== "undefined");
        if (hasDefaultValue) {
            ctx.flags[name] = getDefaultValue(option) ?? defaultValue;
            ctx.defaults[option.name] = true;
            if (typeof option.value === "function") {
                ctx.flags[name] = option.value(ctx.flags[name]);
            }
        }
    }
}
function validateStandaloneOption(ctx, options, optionNames) {
    if (!ctx.standalone || optionNames.length === 1) {
        return;
    }
    // Don't throw an error if all values are coming from the default option.
    for (const [_, opt] of options) {
        if (!ctx.defaults[opt.name] && opt !== ctx.standalone) {
            throw new OptionNotCombinableError(ctx.standalone.name);
        }
    }
}
function validateConflictingOptions(ctx, option) {
    if (!option.conflicts?.length) {
        return;
    }
    for (const flag of option.conflicts) {
        if (isset(flag, ctx.flags)) {
            throw new ConflictingOptionError(option.name, flag);
        }
    }
}
function validateDependingOptions(ctx, option) {
    if (!option.depends) {
        return;
    }
    for (const flag of option.depends) {
        // Don't throw an error if the value is coming from the default option.
        if (!isset(flag, ctx.flags) && !ctx.defaults[option.name]) {
            throw new DependingOptionError(option.name, flag);
        }
    }
}
function validateRequiredValues(ctx, option, name) {
    if (!option.args) {
        return;
    }
    const isArray = option.args.length > 1;
    for (let i = 0; i < option.args.length; i++) {
        const arg = option.args[i];
        if (arg.optional) {
            continue;
        }
        const hasValue = isArray
            ? typeof ctx.flags[name][i] !== "undefined"
            : typeof ctx.flags[name] !== "undefined";
        if (!hasValue) {
            throw new MissingOptionValueError(option.name);
        }
    }
}
function validateRequiredOptions(ctx, options, opts) {
    if (!opts.flags?.length) {
        return;
    }
    const optionsValues = [...options.values()];
    for (const option of opts.flags) {
        if (!option.required || paramCaseToCamelCase(option.name) in ctx.flags) {
            continue;
        }
        const conflicts = option.conflicts ?? [];
        const hasConflict = conflicts.find((flag) => !!ctx.flags[flag]);
        const hasConflicts = hasConflict ||
            optionsValues.find((opt) => opt.conflicts?.find((flag) => flag === option.name));
        if (hasConflicts) {
            continue;
        }
        throw new MissingRequiredOptionError(option.name);
    }
}
/**
 * Check if value exists for flag.
 * @param flagName  Flag name.
 * @param flags     Parsed values.
 */
function isset(flagName, flags) {
    const name = paramCaseToCamelCase(flagName);
    // return typeof values[ name ] !== 'undefined' && values[ name ] !== false;
    return typeof flags[name] !== "undefined";
}
