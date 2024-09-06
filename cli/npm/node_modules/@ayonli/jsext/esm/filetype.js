import { UTIMap } from './filetype/constants.js';

/**
 * Functions to get file types in different fashions.
 * @module
 */
/**
 * Returns the corresponding UTI (Uniform Type Identifier) for the given
 * type, where the type is a file extension or a MIME type name.
 *
 * @example
 * ```ts
 * import { getUTI } from "@ayonli/jsext/filetype";
 *
 * console.log(getUTI(".png")); // public.png
 * console.log(getUTI("image/png")); // public.png
 * ```
 */
function getUTI(type) {
    type = type.toLowerCase();
    if (type.startsWith("*.")) {
        type = type.slice(1);
    }
    if (!type) {
        return undefined;
    }
    for (const [uti, values] of Object.entries(UTIMap)) {
        if (values.includes(type)) {
            return uti;
        }
    }
    return undefined;
}
/**
 * Returns the corresponding MIME type for the given type, where the type is a
 * file extension or a UTI (Uniform Type Identifier) name.
 *
 * @example
 * ```ts
 * import { getMIME } from "@ayonli/jsext/filetype";
 *
 * console.log(getMIME(".png")); // image/png
 * console.log(getMIME("public.png")); // image/png
 * ```
 */
function getMIME(type) {
    type = type.toLowerCase();
    if (type.startsWith("*.")) {
        type = type.slice(1);
    }
    if (!type) {
        return undefined;
    }
    else if (type[0] !== ".") {
        const values = UTIMap[type] || null;
        return values ? values.find(v => v.includes("/")) : undefined;
    }
    for (const values of Object.values(UTIMap)) {
        if (values.includes(type)) {
            const mime = values.find(v => v.includes("/"));
            if (mime) {
                return mime;
            }
        }
    }
    return undefined;
}
/**
 * Returns the corresponding file extensions for the given type, where the type
 * is a MIME type or a UTI (Uniform Type Identifier) name.
 *
 * @example
 * ```ts
 * import { getExtensions } from "@ayonli/jsext/filetype";
 *
 * console.log(getExtensions("image/png")); // [".png"]
 * console.log(getExtensions("public.png")); // [".png"]
 * ```
 */
function getExtensions(type) {
    type = type.toLowerCase();
    if (type.startsWith("*.")) {
        type = type.slice(1);
    }
    if (!type) {
        return [];
    }
    else if (type[0] === ".") {
        return [type];
    }
    if (type[0] !== "." && !type.includes("/")) {
        const values = UTIMap[type] || null;
        return values ? values.filter(v => v[0] === ".") : [];
    }
    if (type.endsWith("/*")) {
        const _type = type.slice(0, -1);
        return Object.values(UTIMap)
            .filter(values => values.some(v => v !== type && v.startsWith(_type)))
            .map(values => values.filter(v => v[0] === "."))
            .flat();
    }
    for (const types of Object.values(UTIMap)) {
        if (types.includes(type)) {
            return types.filter(t => t.startsWith("."));
        }
    }
    return [];
}

export { getExtensions, getMIME, getUTI };
//# sourceMappingURL=filetype.js.map
