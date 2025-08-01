"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const lineForPosition_1 = require("./lineForPosition");
const types_1 = require("./types");
const utils_1 = require("./utils");
exports.getLocationForJsonPath = ({ ast, lineMap, metadata }, path, closest = false) => {
    const node = findNodeAtPath(ast, path, { closest, mergeKeys: metadata !== undefined && metadata.mergeKeys === true });
    if (node === void 0)
        return;
    return getLoc(lineMap, {
        start: getStartPosition(node, lineMap.length > 0 ? lineMap[0] : 0),
        end: getEndPosition(node),
    });
};
function getStartPosition(node, offset) {
    if (node.parent && node.parent.kind === types_1.Kind.MAPPING) {
        if (node.parent.value === null) {
            return node.parent.endPosition;
        }
        if (node.kind !== types_1.Kind.SCALAR) {
            return node.parent.key.endPosition + 1;
        }
    }
    if (node.parent === null && offset - node.startPosition === 0) {
        return 0;
    }
    return node.startPosition;
}
function getEndPosition(node) {
    switch (node.kind) {
        case types_1.Kind.SEQ:
            const { items } = node;
            if (items.length !== 0) {
                const lastItem = items[items.length - 1];
                if (lastItem !== null) {
                    return getEndPosition(lastItem);
                }
            }
            break;
        case types_1.Kind.MAPPING:
            if (node.value !== null) {
                return getEndPosition(node.value);
            }
            break;
        case types_1.Kind.MAP:
            if (node.value !== null && node.mappings.length !== 0) {
                return getEndPosition(node.mappings[node.mappings.length - 1]);
            }
            break;
        case types_1.Kind.SCALAR:
            if (node.parent !== null && node.parent.kind === types_1.Kind.MAPPING && node.parent.value === null) {
                return node.parent.endPosition;
            }
            break;
    }
    return node.endPosition;
}
function findNodeAtPath(node, path, { closest, mergeKeys }) {
    pathLoop: for (const segment of path) {
        if (!utils_1.isObject(node)) {
            return closest ? node : void 0;
        }
        switch (node.kind) {
            case types_1.Kind.MAP:
                const mappings = getMappings(node.mappings, mergeKeys);
                for (let i = mappings.length - 1; i >= 0; i--) {
                    const item = mappings[i];
                    if (item.key.value === segment) {
                        if (item.value === null) {
                            node = item.key;
                        }
                        else {
                            node = item.value;
                        }
                        continue pathLoop;
                    }
                }
                return closest ? node : void 0;
            case types_1.Kind.SEQ:
                for (let i = 0; i < node.items.length; i++) {
                    if (i === Number(segment)) {
                        const item = node.items[i];
                        if (item === null) {
                            break;
                        }
                        node = item;
                        continue pathLoop;
                    }
                }
                return closest ? node : void 0;
            default:
                return closest ? node : void 0;
        }
    }
    return node;
}
function getMappings(mappings, mergeKeys) {
    if (!mergeKeys)
        return mappings;
    return mappings.reduce((mergedMappings, mapping) => {
        if (utils_1.isObject(mapping)) {
            if (mapping.key.value === "<<") {
                mergedMappings.push(...reduceMergeKeys(mapping.value));
            }
            else {
                mergedMappings.push(mapping);
            }
        }
        return mergedMappings;
    }, []);
}
function reduceMergeKeys(node) {
    if (!utils_1.isObject(node))
        return [];
    switch (node.kind) {
        case types_1.Kind.SEQ:
            return node.items.reduceRight((items, item) => {
                items.push(...reduceMergeKeys(item));
                return items;
            }, []);
        case types_1.Kind.MAP:
            return node.mappings;
        case types_1.Kind.ANCHOR_REF:
            return reduceMergeKeys(node.value);
        default:
            return [];
    }
}
const getLoc = (lineMap, { start = 0, end = 0 }) => {
    const startLine = lineForPosition_1.lineForPosition(start, lineMap);
    const endLine = lineForPosition_1.lineForPosition(end, lineMap);
    return {
        range: {
            start: {
                line: startLine,
                character: start - (startLine === 0 ? 0 : lineMap[startLine - 1]),
            },
            end: {
                line: endLine,
                character: end - (endLine === 0 ? 0 : lineMap[endLine - 1]),
            },
        },
    };
};
//# sourceMappingURL=getLocationForJsonPath.js.map