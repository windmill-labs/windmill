"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const types_1 = require("./types");
const utils_1 = require("./utils");
function buildJsonPath(node) {
    const path = [];
    let prevNode = node;
    while (node) {
        switch (node.kind) {
            case types_1.Kind.SCALAR:
                path.unshift(node.value);
                break;
            case types_1.Kind.MAPPING:
                if (prevNode !== node.key) {
                    if (path.length > 0 && utils_1.isObject(node.value) && node.value.value === path[0]) {
                        path[0] = node.key.value;
                    }
                    else {
                        path.unshift(node.key.value);
                    }
                }
                break;
            case types_1.Kind.SEQ:
                if (prevNode) {
                    const index = node.items.indexOf(prevNode);
                    if (prevNode.kind === types_1.Kind.SCALAR) {
                        path[0] = index;
                    }
                    else if (index !== -1) {
                        path.unshift(index);
                    }
                }
                break;
        }
        prevNode = node;
        node = node.parent;
    }
    return path;
}
exports.buildJsonPath = buildJsonPath;
//# sourceMappingURL=buildJsonPath.js.map