import { isNode, isBun, isDeno } from '../env.js';
import { toFsPath } from '../path.js';
import { trim } from '../string.js';
import { interop } from '../module.js';
import { getObjectURL } from '../module/util.js';
import { isFsPath } from '../path/util.js';

const moduleCache = new Map();
function sanitizeModuleId(id, strict = false) {
    let _id = "";
    if (typeof id === "function") {
        let str = id.toString();
        let offset = "import(".length;
        let start = str.lastIndexOf("import(");
        if (start === -1) {
            offset = "require(".length;
            start = str.lastIndexOf("require(");
        }
        if (start === -1) {
            throw new TypeError("the given script is not a dynamic import expression");
        }
        else {
            start += offset;
            const end = str.indexOf(")", start);
            _id = trim(str.slice(start, end), ` '"\'`);
        }
    }
    else {
        _id = id;
    }
    if ((isNode || isBun) && isFsPath(_id)) {
        if (!/\.[cm]?(js|ts|)x?$/.test(_id)) { // if omitted suffix, add suffix
            _id += isBun ? ".ts" : ".js";
        }
        else if (isNode) { // replace .ts/.mts/.cts to .js/.mjs/.cjs in Node.js
            if (_id.endsWith(".ts")) {
                _id = _id.slice(0, -3) + ".js";
            }
            else if (_id.endsWith(".mts")) {
                _id = _id.slice(0, -4) + ".mjs";
            }
            else if (_id.endsWith(".cts")) { // rare, but should support
                _id = _id.slice(0, -4) + ".cjs";
            }
            else if (_id.endsWith(".tsx") || _id.endsWith(".jsx")) { // rare, but should support
                _id = _id.slice(0, -4) + ".js";
            }
        }
    }
    if (!isFsPath(_id) && !strict) {
        _id = "./" + _id;
    }
    return _id;
}
async function resolveModule(modId, baseUrl = undefined) {
    let module;
    if (isNode || isBun) {
        const path = baseUrl ? toFsPath(new URL(modId, baseUrl).href) : modId;
        module = await import(path);
    }
    else {
        const url = new URL(modId, baseUrl).href;
        module = moduleCache.get(url);
        if (!module) {
            if (isDeno) {
                module = await import(url);
                moduleCache.set(url, module);
            }
            else {
                try {
                    module = await import(url);
                    moduleCache.set(url, module);
                }
                catch (err) {
                    if (String(err).includes("Failed")) {
                        const _url = await getObjectURL(url);
                        module = await import(_url);
                        moduleCache.set(url, module);
                    }
                    else {
                        throw err;
                    }
                }
            }
        }
    }
    return interop(module);
}

export { resolveModule, sanitizeModuleId };
//# sourceMappingURL=module.js.map
