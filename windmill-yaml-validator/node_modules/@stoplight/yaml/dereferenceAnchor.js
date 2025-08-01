"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const types_1 = require("./types");
const utils_1 = require("./utils");
exports.dereferenceAnchor = (node, anchorId) => {
    if (!utils_1.isObject(node))
        return node;
    if (node.kind === types_1.Kind.ANCHOR_REF && node.referencesAnchor === anchorId)
        return null;
    switch (node.kind) {
        case types_1.Kind.MAP:
            return Object.assign({}, node, { mappings: node.mappings.map(mapping => exports.dereferenceAnchor(mapping, anchorId)) });
        case types_1.Kind.SEQ:
            return Object.assign({}, node, { items: node.items.map(item => exports.dereferenceAnchor(item, anchorId)) });
        case types_1.Kind.MAPPING:
            return Object.assign({}, node, { value: exports.dereferenceAnchor(node.value, anchorId) });
        case types_1.Kind.SCALAR:
            return node;
        case types_1.Kind.ANCHOR_REF:
            if (utils_1.isObject(node.value) && isSelfReferencingAnchorRef(node)) {
                return null;
            }
            return node;
        default:
            return node;
    }
};
const isSelfReferencingAnchorRef = (anchorRef) => {
    const { referencesAnchor } = anchorRef;
    let node = anchorRef;
    while ((node = node.parent)) {
        if ('anchorId' in node && node.anchorId === referencesAnchor) {
            return true;
        }
    }
    return false;
};
//# sourceMappingURL=dereferenceAnchor.js.map