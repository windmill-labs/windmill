'use strict';
Object.defineProperty(exports, "__esModule", { value: true });
var common = require('./common');
var YAMLException = require('./exception');
var DEFAULT_FULL_SCHEMA = require('./schema/default_full');
var DEFAULT_SAFE_SCHEMA = require('./schema/default_safe');
var _toString = Object.prototype.toString;
var _hasOwnProperty = Object.prototype.hasOwnProperty;
var CHAR_TAB = 0x09;
var CHAR_LINE_FEED = 0x0A;
var CHAR_CARRIAGE_RETURN = 0x0D;
var CHAR_SPACE = 0x20;
var CHAR_EXCLAMATION = 0x21;
var CHAR_DOUBLE_QUOTE = 0x22;
var CHAR_SHARP = 0x23;
var CHAR_PERCENT = 0x25;
var CHAR_AMPERSAND = 0x26;
var CHAR_SINGLE_QUOTE = 0x27;
var CHAR_ASTERISK = 0x2A;
var CHAR_COMMA = 0x2C;
var CHAR_MINUS = 0x2D;
var CHAR_COLON = 0x3A;
var CHAR_EQUALS = 0x3D;
var CHAR_GREATER_THAN = 0x3E;
var CHAR_QUESTION = 0x3F;
var CHAR_COMMERCIAL_AT = 0x40;
var CHAR_LEFT_SQUARE_BRACKET = 0x5B;
var CHAR_RIGHT_SQUARE_BRACKET = 0x5D;
var CHAR_GRAVE_ACCENT = 0x60;
var CHAR_LEFT_CURLY_BRACKET = 0x7B;
var CHAR_VERTICAL_LINE = 0x7C;
var CHAR_RIGHT_CURLY_BRACKET = 0x7D;
var ESCAPE_SEQUENCES = {};
ESCAPE_SEQUENCES[0x00] = '\\0';
ESCAPE_SEQUENCES[0x07] = '\\a';
ESCAPE_SEQUENCES[0x08] = '\\b';
ESCAPE_SEQUENCES[0x09] = '\\t';
ESCAPE_SEQUENCES[0x0A] = '\\n';
ESCAPE_SEQUENCES[0x0B] = '\\v';
ESCAPE_SEQUENCES[0x0C] = '\\f';
ESCAPE_SEQUENCES[0x0D] = '\\r';
ESCAPE_SEQUENCES[0x1B] = '\\e';
ESCAPE_SEQUENCES[0x22] = '\\"';
ESCAPE_SEQUENCES[0x5C] = '\\\\';
ESCAPE_SEQUENCES[0x85] = '\\N';
ESCAPE_SEQUENCES[0xA0] = '\\_';
ESCAPE_SEQUENCES[0x2028] = '\\L';
ESCAPE_SEQUENCES[0x2029] = '\\P';
var DEPRECATED_BOOLEANS_SYNTAX = [
    'y', 'Y', 'yes', 'Yes', 'YES', 'on', 'On', 'ON',
    'n', 'N', 'no', 'No', 'NO', 'off', 'Off', 'OFF'
];
function compileStyleMap(schema, map) {
    var result, keys, index, length, tag, style, type;
    if (map === null)
        return {};
    result = {};
    keys = Object.keys(map);
    for (index = 0, length = keys.length; index < length; index += 1) {
        tag = keys[index];
        style = String(map[tag]);
        if (tag.slice(0, 2) === '!!') {
            tag = 'tag:yaml.org,2002:' + tag.slice(2);
        }
        type = schema.compiledTypeMap['fallback'][tag];
        if (type && _hasOwnProperty.call(type.styleAliases, style)) {
            style = type.styleAliases[style];
        }
        result[tag] = style;
    }
    return result;
}
function encodeHex(character) {
    var string, handle, length;
    string = character.toString(16).toUpperCase();
    if (character <= 0xFF) {
        handle = 'x';
        length = 2;
    }
    else if (character <= 0xFFFF) {
        handle = 'u';
        length = 4;
    }
    else if (character <= 0xFFFFFFFF) {
        handle = 'U';
        length = 8;
    }
    else {
        throw new YAMLException('code point within a string may not be greater than 0xFFFFFFFF');
    }
    return '\\' + handle + common.repeat('0', length - string.length) + string;
}
function State(options) {
    this.schema = options['schema'] || DEFAULT_FULL_SCHEMA;
    this.indent = Math.max(1, (options['indent'] || 2));
    this.noArrayIndent = options['noArrayIndent'] || false;
    this.skipInvalid = options['skipInvalid'] || false;
    this.flowLevel = (common.isNothing(options['flowLevel']) ? -1 : options['flowLevel']);
    this.styleMap = compileStyleMap(this.schema, options['styles'] || null);
    this.sortKeys = options['sortKeys'] || false;
    this.lineWidth = options['lineWidth'] || 80;
    this.noRefs = options['noRefs'] || false;
    this.noCompatMode = options['noCompatMode'] || false;
    this.condenseFlow = options['condenseFlow'] || false;
    this.implicitTypes = this.schema.compiledImplicit;
    this.explicitTypes = this.schema.compiledExplicit;
    this.comments = options['comments'] || {};
    this.tag = null;
    this.result = '';
    this.duplicates = [];
    this.usedDuplicates = null;
}
function indentString(string, spaces) {
    var ind = common.repeat(' ', spaces), position = 0, next = -1, result = '', line, length = string.length;
    while (position < length) {
        next = string.indexOf('\n', position);
        if (next === -1) {
            line = string.slice(position);
            position = length;
        }
        else {
            line = string.slice(position, next + 1);
            position = next + 1;
        }
        if (line.length && line !== '\n')
            result += ind;
        result += line;
    }
    return result;
}
function generateNextLine(state, level) {
    return '\n' + common.repeat(' ', state.indent * level);
}
function testImplicitResolving(state, str) {
    var index, length, type;
    for (index = 0, length = state.implicitTypes.length; index < length; index += 1) {
        type = state.implicitTypes[index];
        if (type.resolve(str)) {
            return true;
        }
    }
    return false;
}
function isWhitespace(c) {
    return c === CHAR_SPACE || c === CHAR_TAB;
}
function isPrintable(c) {
    return (0x00020 <= c && c <= 0x00007E)
        || ((0x000A1 <= c && c <= 0x00D7FF) && c !== 0x2028 && c !== 0x2029)
        || ((0x0E000 <= c && c <= 0x00FFFD) && c !== 0xFEFF)
        || (0x10000 <= c && c <= 0x10FFFF);
}
function isNsChar(c) {
    return isPrintable(c) && !isWhitespace(c)
        && c !== 0xFEFF
        && c !== CHAR_CARRIAGE_RETURN
        && c !== CHAR_LINE_FEED;
}
function isPlainSafe(c, prev) {
    return isPrintable(c) && c !== 0xFEFF
        && c !== CHAR_COMMA
        && c !== CHAR_LEFT_SQUARE_BRACKET
        && c !== CHAR_RIGHT_SQUARE_BRACKET
        && c !== CHAR_LEFT_CURLY_BRACKET
        && c !== CHAR_RIGHT_CURLY_BRACKET
        && c !== CHAR_COLON
        && ((c !== CHAR_SHARP) || (prev && isNsChar(prev)));
}
function isPlainSafeFirst(c) {
    return isPrintable(c) && c !== 0xFEFF
        && !isWhitespace(c)
        && c !== CHAR_MINUS
        && c !== CHAR_QUESTION
        && c !== CHAR_COLON
        && c !== CHAR_COMMA
        && c !== CHAR_LEFT_SQUARE_BRACKET
        && c !== CHAR_RIGHT_SQUARE_BRACKET
        && c !== CHAR_LEFT_CURLY_BRACKET
        && c !== CHAR_RIGHT_CURLY_BRACKET
        && c !== CHAR_SHARP
        && c !== CHAR_AMPERSAND
        && c !== CHAR_ASTERISK
        && c !== CHAR_EXCLAMATION
        && c !== CHAR_VERTICAL_LINE
        && c !== CHAR_EQUALS
        && c !== CHAR_GREATER_THAN
        && c !== CHAR_SINGLE_QUOTE
        && c !== CHAR_DOUBLE_QUOTE
        && c !== CHAR_PERCENT
        && c !== CHAR_COMMERCIAL_AT
        && c !== CHAR_GRAVE_ACCENT;
}
function needIndentIndicator(string) {
    var leadingSpaceRe = /^\n* /;
    return leadingSpaceRe.test(string);
}
var STYLE_PLAIN = 1, STYLE_SINGLE = 2, STYLE_LITERAL = 3, STYLE_FOLDED = 4, STYLE_DOUBLE = 5;
function chooseScalarStyle(string, singleLineOnly, indentPerLevel, lineWidth, testAmbiguousType) {
    var i;
    var char, prev_char;
    var hasLineBreak = false;
    var hasFoldableLine = false;
    var shouldTrackWidth = lineWidth !== -1;
    var previousLineBreak = -1;
    var plain = isPlainSafeFirst(string.charCodeAt(0))
        && !isWhitespace(string.charCodeAt(string.length - 1));
    if (singleLineOnly) {
        for (i = 0; i < string.length; i++) {
            char = string.charCodeAt(i);
            if (!isPrintable(char)) {
                return STYLE_DOUBLE;
            }
            prev_char = i > 0 ? string.charCodeAt(i - 1) : null;
            plain = plain && isPlainSafe(char, prev_char);
        }
    }
    else {
        for (i = 0; i < string.length; i++) {
            char = string.charCodeAt(i);
            if (char === CHAR_LINE_FEED) {
                hasLineBreak = true;
                if (shouldTrackWidth) {
                    hasFoldableLine = hasFoldableLine ||
                        (i - previousLineBreak - 1 > lineWidth &&
                            string[previousLineBreak + 1] !== ' ');
                    previousLineBreak = i;
                }
            }
            else if (!isPrintable(char)) {
                return STYLE_DOUBLE;
            }
            prev_char = i > 0 ? string.charCodeAt(i - 1) : null;
            plain = plain && isPlainSafe(char, prev_char);
        }
        hasFoldableLine = hasFoldableLine || (shouldTrackWidth &&
            (i - previousLineBreak - 1 > lineWidth &&
                string[previousLineBreak + 1] !== ' '));
    }
    if (!hasLineBreak && !hasFoldableLine) {
        return plain && !testAmbiguousType(string)
            ? STYLE_PLAIN : STYLE_SINGLE;
    }
    if (indentPerLevel > 9 && needIndentIndicator(string)) {
        return STYLE_DOUBLE;
    }
    return hasFoldableLine ? STYLE_FOLDED : STYLE_LITERAL;
}
function writeScalar(state, string, level, iskey, pointer) {
    var _result = (function () {
        if (string.length === 0) {
            return "''";
        }
        if (!state.noCompatMode &&
            DEPRECATED_BOOLEANS_SYNTAX.indexOf(string) !== -1) {
            return "'" + string + "'";
        }
        var indent = state.indent * Math.max(1, level);
        var lineWidth = state.lineWidth === -1
            ? -1 : Math.max(Math.min(state.lineWidth, 40), state.lineWidth - indent);
        var singleLineOnly = iskey
            || (state.flowLevel > -1 && level >= state.flowLevel);
        function testAmbiguity(string) {
            return testImplicitResolving(state, string);
        }
        switch (chooseScalarStyle(string, singleLineOnly, state.indent, lineWidth, testAmbiguity)) {
            case STYLE_PLAIN:
                return string;
            case STYLE_SINGLE:
                return "'" + string.replace(/'/g, "''") + "'";
            case STYLE_LITERAL:
                return '|' + blockHeader(string, state.indent)
                    + dropEndingNewline(indentString(string, indent));
            case STYLE_FOLDED:
                return '>' + blockHeader(string, state.indent)
                    + dropEndingNewline(indentString(foldString(string, lineWidth), indent));
            case STYLE_DOUBLE:
                return '"' + escapeString(string) + '"';
            default:
                throw new YAMLException('impossible error: invalid scalar style');
        }
    }());
    if (!iskey) {
        let comments = new Comments(state, pointer);
        let comment = comments.write(level, 'before-eol');
        if (comment !== '') {
            _result += ' ' + comment;
        }
    }
    state.dump = _result;
}
function blockHeader(string, indentPerLevel) {
    var indentIndicator = needIndentIndicator(string) ? String(indentPerLevel) : '';
    var clip = string[string.length - 1] === '\n';
    var keep = clip && (string[string.length - 2] === '\n' || string === '\n');
    var chomp = keep ? '+' : (clip ? '' : '-');
    return indentIndicator + chomp + '\n';
}
function dropEndingNewline(string) {
    return string[string.length - 1] === '\n' ? string.slice(0, -1) : string;
}
function foldString(string, width) {
    var lineRe = /(\n+)([^\n]*)/g;
    var result = (function () {
        var nextLF = string.indexOf('\n');
        nextLF = nextLF !== -1 ? nextLF : string.length;
        lineRe.lastIndex = nextLF;
        return foldLine(string.slice(0, nextLF), width);
    }());
    var prevMoreIndented = string[0] === '\n' || string[0] === ' ';
    var moreIndented;
    var match;
    while ((match = lineRe.exec(string))) {
        var prefix = match[1], line = match[2];
        moreIndented = (line[0] === ' ');
        result += prefix
            + (!prevMoreIndented && !moreIndented && line !== ''
                ? '\n' : '')
            + foldLine(line, width);
        prevMoreIndented = moreIndented;
    }
    return result;
}
function foldLine(line, width) {
    if (line === '' || line[0] === ' ')
        return line;
    var breakRe = / [^ ]/g;
    var match;
    var start = 0, end, curr = 0, next = 0;
    var result = '';
    while ((match = breakRe.exec(line))) {
        next = match.index;
        if (next - start > width) {
            end = (curr > start) ? curr : next;
            result += '\n' + line.slice(start, end);
            start = end + 1;
        }
        curr = next;
    }
    result += '\n';
    if (line.length - start > width && curr > start) {
        result += line.slice(start, curr) + '\n' + line.slice(curr + 1);
    }
    else {
        result += line.slice(start);
    }
    return result.slice(1);
}
function escapeString(string) {
    var result = '';
    var char, nextChar;
    var escapeSeq;
    for (var i = 0; i < string.length; i++) {
        char = string.charCodeAt(i);
        if (char >= 0xD800 && char <= 0xDBFF) {
            nextChar = string.charCodeAt(i + 1);
            if (nextChar >= 0xDC00 && nextChar <= 0xDFFF) {
                result += encodeHex((char - 0xD800) * 0x400 + nextChar - 0xDC00 + 0x10000);
                i++;
                continue;
            }
        }
        escapeSeq = ESCAPE_SEQUENCES[char];
        result += !escapeSeq && isPrintable(char)
            ? string[i]
            : escapeSeq || encodeHex(char);
    }
    return result;
}
function writeFlowSequence(state, level, object, pointer) {
    var _result = '', _tag = state.tag, index, length;
    for (index = 0, length = object.length; index < length; index += 1) {
        if (writeNode(state, level, object[index], false, false, false, pointer)) {
            if (index !== 0)
                _result += ',' + (!state.condenseFlow ? ' ' : '');
            _result += state.dump;
        }
    }
    state.tag = _tag;
    state.dump = '[' + _result + ']';
}
function writeBlockSequence(state, level, object, compact, pointer) {
    var _result = '', _tag = state.tag, index, length;
    var comments = new Comments(state, pointer);
    _result += comments.write(level, 'before-eol');
    _result += comments.write(level, 'leading');
    for (index = 0, length = object.length; index < length; index += 1) {
        _result += comments.writeAt(String(index), level, 'before');
        if (writeNode(state, level + 1, object[index], true, true, false, `${pointer}/${index}`)) {
            if (!compact || index !== 0) {
                _result += generateNextLine(state, level);
            }
            if (state.dump && CHAR_LINE_FEED === state.dump.charCodeAt(0)) {
                _result += '-';
            }
            else {
                _result += '- ';
            }
            _result += state.dump;
        }
        _result += comments.writeAt(String(index), level, 'after');
    }
    state.tag = _tag;
    state.dump = _result || '[]';
    state.dump += comments.write(level, 'trailing');
}
function writeFlowMapping(state, level, object, pointer) {
    var _result = '', _tag = state.tag, objectKeyList = Object.keys(object), index, length, objectKey, objectValue, pairBuffer;
    for (index = 0, length = objectKeyList.length; index < length; index += 1) {
        pairBuffer = '';
        if (index !== 0)
            pairBuffer += ', ';
        if (state.condenseFlow)
            pairBuffer += '"';
        objectKey = objectKeyList[index];
        objectValue = object[objectKey];
        if (!writeNode(state, level, objectKey, false, false, false, pointer)) {
            continue;
        }
        if (state.dump.length > 1024)
            pairBuffer += '? ';
        pairBuffer += state.dump + (state.condenseFlow ? '"' : '') + ':' + (state.condenseFlow ? '' : ' ');
        if (!writeNode(state, level, objectValue, false, false, false, pointer)) {
            continue;
        }
        pairBuffer += state.dump;
        _result += pairBuffer;
    }
    state.tag = _tag;
    state.dump = '{' + _result + '}';
}
function writeBlockMapping(state, level, object, compact, pointer) {
    var _result = '', _tag = state.tag, objectKeyList = Object.keys(object), index, length, objectKey, objectValue, explicitPair, pairBuffer;
    if (state.sortKeys === true) {
        objectKeyList.sort();
    }
    else if (typeof state.sortKeys === 'function') {
        objectKeyList.sort(state.sortKeys);
    }
    else if (state.sortKeys) {
        throw new YAMLException('sortKeys must be a boolean or a function');
    }
    var comments = new Comments(state, pointer);
    _result += comments.write(level, 'before-eol');
    _result += comments.write(level, 'leading');
    for (index = 0, length = objectKeyList.length; index < length; index += 1) {
        pairBuffer = '';
        if (!compact || index !== 0) {
            pairBuffer += generateNextLine(state, level);
        }
        objectKey = objectKeyList[index];
        objectValue = object[objectKey];
        _result += comments.writeAt(objectKey, level, 'before');
        if (!writeNode(state, level + 1, objectKey, true, true, true, pointer)) {
            continue;
        }
        explicitPair = (state.tag !== null && state.tag !== '?') ||
            (state.dump && state.dump.length > 1024);
        if (explicitPair) {
            if (state.dump && CHAR_LINE_FEED === state.dump.charCodeAt(0)) {
                pairBuffer += '?';
            }
            else {
                pairBuffer += '? ';
            }
        }
        pairBuffer += state.dump;
        if (explicitPair) {
            pairBuffer += generateNextLine(state, level);
        }
        if (!writeNode(state, level + 1, objectValue, true, explicitPair, false, `${pointer}/${encodeSegment(objectKey)}`)) {
            continue;
        }
        if (state.dump && CHAR_LINE_FEED === state.dump.charCodeAt(0)) {
            pairBuffer += ':';
        }
        else {
            pairBuffer += ': ';
        }
        pairBuffer += state.dump;
        _result += pairBuffer;
        _result += comments.writeAt(level, objectKey, 'after');
    }
    state.tag = _tag;
    state.dump = _result || '{}';
    state.dump += comments.write(level, 'trailing');
}
function detectType(state, object, explicit) {
    var _result, typeList, index, length, type, style;
    typeList = explicit ? state.explicitTypes : state.implicitTypes;
    for (index = 0, length = typeList.length; index < length; index += 1) {
        type = typeList[index];
        if ((type.instanceOf || type.predicate) &&
            (!type.instanceOf || ((typeof object === 'object') && (object instanceof type.instanceOf))) &&
            (!type.predicate || type.predicate(object))) {
            state.tag = explicit ? type.tag : '?';
            if (type.represent) {
                style = state.styleMap[type.tag] || type.defaultStyle;
                if (_toString.call(type.represent) === '[object Function]') {
                    _result = type.represent(object, style);
                }
                else if (_hasOwnProperty.call(type.represent, style)) {
                    _result = type.represent[style](object, style);
                }
                else {
                    throw new YAMLException('!<' + type.tag + '> tag resolver accepts not "' + style + '" style');
                }
                state.dump = _result;
            }
            return true;
        }
    }
    return false;
}
function writeNode(state, level, object, block, compact, iskey, pointer) {
    state.tag = null;
    state.dump = object;
    if (!detectType(state, object, false)) {
        detectType(state, object, true);
    }
    var type = _toString.call(state.dump);
    if (block) {
        block = (state.flowLevel < 0 || state.flowLevel > level);
    }
    if ((state.tag !== null && state.tag !== '?') || (state.indent !== 2 && level > 0)) {
        compact = false;
    }
    var objectOrArray = type === '[object Object]' || type === '[object Array]', duplicateIndex, duplicate;
    if (objectOrArray) {
        duplicateIndex = state.duplicates.indexOf(object);
        duplicate = duplicateIndex !== -1;
    }
    if ((state.tag !== null && state.tag !== '?') || duplicate || (state.indent !== 2 && level > 0)) {
        compact = false;
    }
    if (duplicate && state.usedDuplicates[duplicateIndex]) {
        state.dump = '*ref_' + duplicateIndex;
    }
    else {
        if (objectOrArray && duplicate && !state.usedDuplicates[duplicateIndex]) {
            state.usedDuplicates[duplicateIndex] = true;
        }
        if (type === '[object Object]') {
            if (block && (Object.keys(state.dump).length !== 0)) {
                writeBlockMapping(state, level, state.dump, compact, pointer);
                if (duplicate) {
                    state.dump = '&ref_' + duplicateIndex + state.dump;
                }
            }
            else {
                writeFlowMapping(state, level, state.dump, pointer);
                if (duplicate) {
                    state.dump = '&ref_' + duplicateIndex + ' ' + state.dump;
                }
            }
        }
        else if (type === '[object Array]') {
            var arrayLevel = (state.noArrayIndent && (level > 0)) ? level - 1 : level;
            if (block && (state.dump.length !== 0)) {
                writeBlockSequence(state, arrayLevel, state.dump, compact, pointer);
                if (duplicate) {
                    state.dump = '&ref_' + duplicateIndex + state.dump;
                }
            }
            else {
                writeFlowSequence(state, arrayLevel, state.dump, pointer);
                if (duplicate) {
                    state.dump = '&ref_' + duplicateIndex + ' ' + state.dump;
                }
            }
        }
        else if (type === '[object String]') {
            if (state.tag !== '?') {
                writeScalar(state, state.dump, level, iskey, pointer);
            }
        }
        else {
            if (state.skipInvalid)
                return false;
            throw new YAMLException('unacceptable kind of an object to dump ' + type);
        }
        if (state.tag !== null && state.tag !== '?') {
            state.dump = '!<' + state.tag + '> ' + state.dump;
        }
    }
    return true;
}
function getDuplicateReferences(object, state) {
    var objects = [], duplicatesIndexes = [], index, length;
    inspectNode(object, objects, duplicatesIndexes);
    for (index = 0, length = duplicatesIndexes.length; index < length; index += 1) {
        state.duplicates.push(objects[duplicatesIndexes[index]]);
    }
    state.usedDuplicates = new Array(length);
}
function inspectNode(object, objects, duplicatesIndexes) {
    var objectKeyList, index, length;
    if (object !== null && typeof object === 'object') {
        index = objects.indexOf(object);
        if (index !== -1) {
            if (duplicatesIndexes.indexOf(index) === -1) {
                duplicatesIndexes.push(index);
            }
        }
        else {
            objects.push(object);
            if (Array.isArray(object)) {
                for (index = 0, length = object.length; index < length; index += 1) {
                    inspectNode(object[index], objects, duplicatesIndexes);
                }
            }
            else {
                objectKeyList = Object.keys(object);
                for (index = 0, length = objectKeyList.length; index < length; index += 1) {
                    inspectNode(object[objectKeyList[index]], objects, duplicatesIndexes);
                }
            }
        }
    }
}
function dump(input, options) {
    options = options || {};
    var state = new State(options);
    if (!options.noRefs)
        getDuplicateReferences(input, state);
    if (writeNode(state, 0, input, true, true, false, '#')) {
        return state.dump + '\n';
    }
    return '';
}
exports.dump = dump;
function safeDump(input, options) {
    return dump(input, common.extend({ schema: DEFAULT_SAFE_SCHEMA }, options));
}
exports.safeDump = safeDump;
const TILDE_REGEXP = /~/g;
const SLASH_REGEXP = /\//g;
function encodeSegment(input) {
    return input.replace(TILDE_REGEXP, '~0').replace(SLASH_REGEXP, '~1');
}
function Comments(state, pointer) {
    this.state = state;
    this.comments = {
        'before-eol': new Set(),
        leading: new Set(),
        trailing: new Set(),
        before: new Map(),
        after: new Map(),
    };
    this.written = new WeakSet();
    if (state.comments !== null && pointer in state.comments) {
        for (let comment of state.comments[pointer]) {
            switch (comment.placement) {
                case 'before-eol':
                case 'leading':
                case 'trailing':
                    this.comments[comment.placement].add(comment);
                    break;
                case 'between':
                    let before = this.comments.before.get(comment.between[1]);
                    if (!before) {
                        this.comments.before.set(comment.between[1], new Set([comment]));
                    }
                    else {
                        before.add(comment);
                    }
                    let after = this.comments.after.get(comment.between[0]);
                    if (!after) {
                        this.comments.after.set(comment.between[0], new Set([comment]));
                    }
                    else {
                        after.add(comment);
                    }
                    break;
            }
        }
    }
}
Comments.prototype.write = function (level, placement) {
    let result = '';
    for (let comment of this.comments[placement]) {
        result += this._write(comment, level);
    }
    return result;
};
Comments.prototype.writeAt = function (key, level, placement) {
    let result = '';
    let comments = this.comments[placement].get(key);
    if (comments) {
        for (let comment of comments) {
            result += this._write(comment, level);
        }
    }
    return result;
};
Comments.prototype._write = function (comment, level) {
    if (this.written.has(comment))
        return '';
    this.written.add(comment);
    let result = '#' + comment.value;
    if (comment.placement === 'before-eol') {
        return result;
    }
    else if (level === 0 && comment.placement === 'leading') {
        return result + '\n';
    }
    else {
        return generateNextLine(this.state, level) + result;
    }
};
//# sourceMappingURL=dumper.js.map