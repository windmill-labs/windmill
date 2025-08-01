"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const buildJsonPath_1 = require("./buildJsonPath");
const types_1 = require("./types");
const utils_1 = require("./utils");
exports.getJsonPathForPosition = ({ ast, lineMap }, { line, character }) => {
    if (line >= lineMap.length || character >= lineMap[line]) {
        return;
    }
    const startOffset = line === 0 ? 0 : lineMap[line - 1] + 1;
    const node = findClosestScalar(ast, Math.min(lineMap[line] - 1, startOffset + character), line, lineMap);
    if (!utils_1.isObject(node))
        return;
    const path = buildJsonPath_1.buildJsonPath(node);
    if (path.length === 0)
        return;
    return path;
};
function* walk(node) {
    switch (node.kind) {
        case types_1.Kind.MAP:
            if (node.mappings.length !== 0) {
                for (const mapping of node.mappings) {
                    if (utils_1.isObject(mapping)) {
                        yield mapping;
                    }
                }
            }
            break;
        case types_1.Kind.MAPPING:
            if (utils_1.isObject(node.key)) {
                yield node.key;
            }
            if (utils_1.isObject(node.value)) {
                yield node.value;
            }
            break;
        case types_1.Kind.SEQ:
            if (node.items.length !== 0) {
                for (const item of node.items) {
                    if (utils_1.isObject(item)) {
                        yield item;
                    }
                }
            }
            break;
        case types_1.Kind.SCALAR:
            yield node;
            break;
    }
}
function getFirstScalarChild(node, line, lineMap) {
    const startOffset = lineMap[line - 1] + 1;
    const endOffset = lineMap[line];
    switch (node.kind) {
        case types_1.Kind.MAPPING:
            return node.key;
        case types_1.Kind.MAP:
            if (node.mappings.length !== 0) {
                for (const mapping of node.mappings) {
                    if (mapping.startPosition > startOffset && mapping.startPosition <= endOffset) {
                        return getFirstScalarChild(mapping, line, lineMap);
                    }
                }
            }
            break;
        case types_1.Kind.SEQ:
            if (node.items.length !== 0) {
                for (const item of node.items) {
                    if (item !== null && item.startPosition > startOffset && item.startPosition <= endOffset) {
                        return getFirstScalarChild(item, line, lineMap);
                    }
                }
            }
            break;
    }
    return node;
}
function findClosestScalar(container, offset, line, lineMap) {
    for (const node of walk(container)) {
        if (node.startPosition <= offset && offset <= node.endPosition) {
            return node.kind === types_1.Kind.SCALAR ? node : findClosestScalar(node, offset, line, lineMap);
        }
    }
    if (lineMap[line - 1] === lineMap[line] - 1) {
        return container;
    }
    if (container.startPosition < lineMap[line - 1] && offset <= container.endPosition) {
        if (container.kind !== types_1.Kind.MAPPING) {
            return getFirstScalarChild(container, line, lineMap);
        }
        if (container.value && container.key.endPosition < offset) {
            return getFirstScalarChild(container.value, line, lineMap);
        }
    }
    return container;
}
//# sourceMappingURL=getJsonPathForPosition.js.map