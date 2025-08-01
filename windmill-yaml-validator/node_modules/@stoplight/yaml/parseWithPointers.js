"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const ordered_object_literal_1 = require("@stoplight/ordered-object-literal");
const types_1 = require("@stoplight/types");
const yaml_ast_parser_1 = require("@stoplight/yaml-ast-parser");
const buildJsonPath_1 = require("./buildJsonPath");
const dereferenceAnchor_1 = require("./dereferenceAnchor");
const lineForPosition_1 = require("./lineForPosition");
const types_2 = require("./types");
const utils_1 = require("./utils");
exports.parseWithPointers = (value, options) => {
    const lineMap = computeLineMap(value);
    const ast = yaml_ast_parser_1.load(value, Object.assign({}, options, { ignoreDuplicateKeys: true }));
    const parsed = {
        ast,
        lineMap,
        data: undefined,
        diagnostics: [],
        metadata: options,
        comments: {},
    };
    if (!ast)
        return parsed;
    const normalizedOptions = normalizeOptions(options);
    const comments = new Comments(parsed.comments, Comments.mapComments(normalizedOptions.attachComments && ast.comments ? ast.comments : [], lineMap), ast, lineMap, '#');
    const ctx = {
        lineMap,
        diagnostics: parsed.diagnostics,
    };
    parsed.data = walkAST(ctx, ast, comments, normalizedOptions);
    if (ast.errors) {
        parsed.diagnostics.push(...transformErrors(ast.errors, lineMap));
    }
    if (parsed.diagnostics.length > 0) {
        parsed.diagnostics.sort((itemA, itemB) => itemA.range.start.line - itemB.range.start.line);
    }
    if (Array.isArray(parsed.ast.errors)) {
        parsed.ast.errors.length = 0;
    }
    return parsed;
};
const TILDE_REGEXP = /~/g;
const SLASH_REGEXP = /\//g;
function encodeSegment(input) {
    return input.replace(TILDE_REGEXP, '~0').replace(SLASH_REGEXP, '~1');
}
const walkAST = (ctx, node, comments, options) => {
    if (node) {
        switch (node.kind) {
            case types_2.Kind.MAP: {
                const mapComments = comments.enter(node);
                const { lineMap, diagnostics } = ctx;
                const { preserveKeyOrder, ignoreDuplicateKeys, json, mergeKeys } = options;
                const container = createMapContainer(preserveKeyOrder);
                const seenKeys = [];
                const handleMergeKeys = mergeKeys;
                const yamlMode = !json;
                const handleDuplicates = !ignoreDuplicateKeys;
                for (const mapping of node.mappings) {
                    if (!validateMappingKey(mapping, lineMap, diagnostics, yamlMode))
                        continue;
                    const key = String(getScalarValue(mapping.key));
                    const mappingComments = mapComments.enter(mapping, encodeSegment(key));
                    if ((yamlMode || handleDuplicates) && (!handleMergeKeys || key !== "<<")) {
                        if (seenKeys.includes(key)) {
                            if (yamlMode) {
                                throw new Error('Duplicate YAML mapping key encountered');
                            }
                            if (handleDuplicates) {
                                diagnostics.push(createYAMLException(mapping.key, lineMap, 'duplicate key'));
                            }
                        }
                        else {
                            seenKeys.push(key);
                        }
                    }
                    if (handleMergeKeys && key === "<<") {
                        const reduced = reduceMergeKeys(walkAST(ctx, mapping.value, mappingComments, options), preserveKeyOrder);
                        Object.assign(container, reduced);
                    }
                    else {
                        container[key] = walkAST(ctx, mapping.value, mappingComments, options);
                        if (preserveKeyOrder) {
                            pushKey(container, key);
                        }
                    }
                    mappingComments.attachComments();
                }
                mapComments.attachComments();
                return container;
            }
            case types_2.Kind.SEQ: {
                const nodeComments = comments.enter(node);
                const container = node.items.map((item, i) => {
                    if (item !== null) {
                        const sequenceItemComments = nodeComments.enter(item, i);
                        const walked = walkAST(ctx, item, sequenceItemComments, options);
                        sequenceItemComments.attachComments();
                        return walked;
                    }
                    else {
                        return null;
                    }
                });
                nodeComments.attachComments();
                return container;
            }
            case types_2.Kind.SCALAR: {
                const value = getScalarValue(node);
                return !options.bigInt && typeof value === 'bigint' ? Number(value) : value;
            }
            case types_2.Kind.ANCHOR_REF: {
                if (utils_1.isObject(node.value)) {
                    node.value = dereferenceAnchor_1.dereferenceAnchor(node.value, node.referencesAnchor);
                }
                return walkAST(ctx, node.value, comments, options);
            }
            default:
                return null;
        }
    }
    return node;
};
function getScalarValue(node) {
    switch (yaml_ast_parser_1.determineScalarType(node)) {
        case types_2.ScalarType.null:
            return null;
        case types_2.ScalarType.string:
            return String(node.value);
        case types_2.ScalarType.bool:
            return yaml_ast_parser_1.parseYamlBoolean(node.value);
        case types_2.ScalarType.int:
            return yaml_ast_parser_1.parseYamlBigInteger(node.value);
        case types_2.ScalarType.float:
            return yaml_ast_parser_1.parseYamlFloat(node.value);
    }
}
const computeLineMap = (input) => {
    const lineMap = [];
    let i = 0;
    for (; i < input.length; i++) {
        if (input[i] === '\n') {
            lineMap.push(i + 1);
        }
    }
    lineMap.push(i + 1);
    return lineMap;
};
function getLineLength(lineMap, line) {
    if (line === 0) {
        return Math.max(0, lineMap[0] - 1);
    }
    return Math.max(0, lineMap[line] - lineMap[line - 1] - 1);
}
const transformErrors = (errors, lineMap) => {
    const validations = [];
    let possiblyUnexpectedFlow = -1;
    let i = 0;
    for (const error of errors) {
        const validation = {
            code: error.name,
            message: error.reason,
            severity: error.isWarning ? types_1.DiagnosticSeverity.Warning : types_1.DiagnosticSeverity.Error,
            range: {
                start: {
                    line: error.mark.line,
                    character: error.mark.column,
                },
                end: {
                    line: error.mark.line,
                    character: error.mark.toLineEnd ? getLineLength(lineMap, error.mark.line) : error.mark.column,
                },
            },
        };
        const isBrokenFlow = error.reason === 'missed comma between flow collection entries';
        if (isBrokenFlow) {
            possiblyUnexpectedFlow = possiblyUnexpectedFlow === -1 ? i : possiblyUnexpectedFlow;
        }
        else if (possiblyUnexpectedFlow !== -1) {
            validations[possiblyUnexpectedFlow].range.end = validation.range.end;
            validations[possiblyUnexpectedFlow].message = 'invalid mixed usage of block and flow styles';
            validations.length = possiblyUnexpectedFlow + 1;
            i = validations.length;
            possiblyUnexpectedFlow = -1;
        }
        validations.push(validation);
        i++;
    }
    return validations;
};
const reduceMergeKeys = (items, preserveKeyOrder) => {
    if (Array.isArray(items)) {
        const reduced = items.reduceRight(preserveKeyOrder
            ? (merged, item) => {
                const keys = Object.keys(item);
                Object.assign(merged, item);
                for (let i = keys.length - 1; i >= 0; i--) {
                    unshiftKey(merged, keys[i]);
                }
                return merged;
            }
            : (merged, item) => Object.assign(merged, item), createMapContainer(preserveKeyOrder));
        return reduced;
    }
    return typeof items !== 'object' || items === null ? null : Object(items);
};
function createMapContainer(preserveKeyOrder) {
    return preserveKeyOrder ? ordered_object_literal_1.default({}) : {};
}
function deleteKey(container, key) {
    if (!(key in container))
        return;
    const order = ordered_object_literal_1.getOrder(container);
    const index = order.indexOf(key);
    if (index !== -1) {
        order.splice(index, 1);
    }
}
function unshiftKey(container, key) {
    deleteKey(container, key);
    ordered_object_literal_1.getOrder(container).unshift(key);
}
function pushKey(container, key) {
    deleteKey(container, key);
    ordered_object_literal_1.getOrder(container).push(key);
}
function validateMappingKey(mapping, lineMap, diagnostics, yamlMode) {
    if (mapping.key.kind !== types_2.Kind.SCALAR) {
        if (!yamlMode) {
            diagnostics.push(createYAMLIncompatibilityException(mapping.key, lineMap, 'mapping key must be a string scalar', yamlMode));
        }
        return false;
    }
    if (!yamlMode) {
        const type = typeof getScalarValue(mapping.key);
        if (type !== 'string') {
            diagnostics.push(createYAMLIncompatibilityException(mapping.key, lineMap, `mapping key must be a string scalar rather than ${mapping.key.valueObject === null ? 'null' : type}`, yamlMode));
        }
    }
    return true;
}
function createYAMLIncompatibilityException(node, lineMap, message, yamlMode) {
    const exception = createYAMLException(node, lineMap, message);
    exception.code = 'YAMLIncompatibleValue';
    exception.severity = yamlMode ? types_1.DiagnosticSeverity.Hint : types_1.DiagnosticSeverity.Warning;
    return exception;
}
function createYAMLException(node, lineMap, message) {
    return {
        code: 'YAMLException',
        message,
        severity: types_1.DiagnosticSeverity.Error,
        path: buildJsonPath_1.buildJsonPath(node),
        range: getRange(lineMap, node.startPosition, node.endPosition),
    };
}
function getRange(lineMap, startPosition, endPosition) {
    const startLine = lineForPosition_1.lineForPosition(startPosition, lineMap);
    const endLine = lineForPosition_1.lineForPosition(endPosition, lineMap);
    return {
        start: {
            line: startLine,
            character: startLine === 0 ? startPosition : startPosition - lineMap[startLine - 1],
        },
        end: {
            line: endLine,
            character: endLine === 0 ? endPosition : endPosition - lineMap[endLine - 1],
        },
    };
}
class Comments {
    constructor(attachedComments, comments, node, lineMap, pointer) {
        this.attachedComments = attachedComments;
        this.node = node;
        this.lineMap = lineMap;
        this.pointer = pointer;
        if (comments.length === 0) {
            this.comments = [];
        }
        else {
            const startPosition = this.getStartPosition(node);
            const endPosition = this.getEndPosition(node);
            const startLine = lineForPosition_1.lineForPosition(startPosition, this.lineMap);
            const endLine = lineForPosition_1.lineForPosition(endPosition, this.lineMap);
            const matchingComments = [];
            for (let i = comments.length - 1; i >= 0; i--) {
                const comment = comments[i];
                if (comment.range.start.line >= startLine && comment.range.end.line <= endLine) {
                    matchingComments.push(comment);
                    comments.splice(i, 1);
                }
            }
            this.comments = matchingComments;
        }
    }
    getStartPosition(node) {
        if (node.parent === null) {
            return 0;
        }
        return node.kind === types_2.Kind.MAPPING ? node.key.startPosition : node.startPosition;
    }
    getEndPosition(node) {
        switch (node.kind) {
            case types_2.Kind.MAPPING:
                return node.value === null ? node.endPosition : this.getEndPosition(node.value);
            case types_2.Kind.MAP:
                return node.mappings.length === 0 ? node.endPosition : node.mappings[node.mappings.length - 1].endPosition;
            case types_2.Kind.SEQ: {
                if (node.items.length === 0) {
                    return node.endPosition;
                }
                const lastItem = node.items[node.items.length - 1];
                return lastItem === null ? node.endPosition : lastItem.endPosition;
            }
            default:
                return node.endPosition;
        }
    }
    static mapComments(comments, lineMap) {
        return comments.map(comment => ({
            value: comment.value,
            range: getRange(lineMap, comment.startPosition, comment.endPosition),
            startPosition: comment.startPosition,
            endPosition: comment.endPosition,
        }));
    }
    enter(node, key) {
        return new Comments(this.attachedComments, this.comments, node, this.lineMap, key === void 0 ? this.pointer : `${this.pointer}/${key}`);
    }
    static isLeading(node, startPosition) {
        switch (node.kind) {
            case types_2.Kind.MAP:
                return node.mappings.length === 0 || node.mappings[0].startPosition > startPosition;
            case types_2.Kind.SEQ: {
                if (node.items.length === 0) {
                    return true;
                }
                const firstItem = node.items[0];
                return firstItem === null || firstItem.startPosition > startPosition;
            }
            case types_2.Kind.MAPPING:
                return node.value === null || node.value.startPosition > startPosition;
            default:
                return false;
        }
    }
    static isTrailing(node, endPosition) {
        switch (node.kind) {
            case types_2.Kind.MAP:
                return node.mappings.length > 0 && endPosition > node.mappings[node.mappings.length - 1].endPosition;
            case types_2.Kind.SEQ:
                if (node.items.length === 0) {
                    return false;
                }
                const lastItem = node.items[node.items.length - 1];
                return lastItem !== null && endPosition > lastItem.endPosition;
            case types_2.Kind.MAPPING:
                return node.value !== null && endPosition > node.value.endPosition;
            default:
                return false;
        }
    }
    static findBetween(node, startPosition, endPosition) {
        switch (node.kind) {
            case types_2.Kind.MAP: {
                let left;
                for (const mapping of node.mappings) {
                    if (startPosition > mapping.startPosition) {
                        left = mapping.key.value;
                    }
                    else if (left !== void 0 && mapping.startPosition > endPosition) {
                        return [left, mapping.key.value];
                    }
                }
                return null;
            }
            case types_2.Kind.SEQ: {
                let left;
                for (let i = 0; i < node.items.length; i++) {
                    const item = node.items[i];
                    if (item === null)
                        continue;
                    if (startPosition > item.startPosition) {
                        left = String(i);
                    }
                    else if (left !== void 0 && item.startPosition > endPosition) {
                        return [left, String(i)];
                    }
                }
                return null;
            }
            default:
                return null;
        }
    }
    isBeforeEOL(comment) {
        return (this.node.kind === types_2.Kind.SCALAR ||
            (this.node.kind === types_2.Kind.MAPPING &&
                comment.range.end.line === lineForPosition_1.lineForPosition(this.node.key.endPosition, this.lineMap)));
    }
    attachComments() {
        if (this.comments.length === 0)
            return;
        const attachedComments = (this.attachedComments[this.pointer] = this.attachedComments[this.pointer] || []);
        for (const comment of this.comments) {
            if (this.isBeforeEOL(comment)) {
                attachedComments.push({
                    value: comment.value,
                    placement: 'before-eol',
                });
            }
            else if (Comments.isLeading(this.node, comment.startPosition)) {
                attachedComments.push({
                    value: comment.value,
                    placement: 'leading',
                });
            }
            else if (Comments.isTrailing(this.node, comment.endPosition)) {
                attachedComments.push({
                    value: comment.value,
                    placement: 'trailing',
                });
            }
            else {
                const between = Comments.findBetween(this.node, comment.startPosition, comment.endPosition);
                if (between !== null) {
                    attachedComments.push({
                        value: comment.value,
                        placement: 'between',
                        between,
                    });
                }
                else {
                    attachedComments.push({
                        value: comment.value,
                        placement: 'trailing',
                    });
                }
            }
        }
    }
}
function normalizeOptions(options) {
    if (options === void 0) {
        return {
            attachComments: false,
            preserveKeyOrder: false,
            bigInt: false,
            mergeKeys: false,
            json: true,
            ignoreDuplicateKeys: false,
        };
    }
    return Object.assign({}, options, { attachComments: options.attachComments === true, preserveKeyOrder: options.preserveKeyOrder === true, bigInt: options.bigInt === true, mergeKeys: options.mergeKeys === true, json: options.json !== false, ignoreDuplicateKeys: options.ignoreDuplicateKeys !== false });
}
//# sourceMappingURL=parseWithPointers.js.map