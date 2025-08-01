"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.lineForPosition = (pos, lines, start = 0, end) => {
    if (pos === 0 || lines.length === 0 || pos < lines[0]) {
        return 0;
    }
    if (typeof end === 'undefined') {
        end = lines.length;
    }
    const target = Math.floor((end - start) / 2) + start;
    if (pos >= lines[target] && !lines[target + 1]) {
        return target + 1;
    }
    const nextLinePos = lines[Math.min(target + 1, lines.length)];
    if (pos === lines[target] - 1) {
        return target;
    }
    if (pos >= lines[target] && pos <= nextLinePos) {
        if (pos === nextLinePos) {
            return target + 2;
        }
        return target + 1;
    }
    if (pos > lines[target]) {
        return exports.lineForPosition(pos, lines, target + 1, end);
    }
    else {
        return exports.lineForPosition(pos, lines, start, target - 1);
    }
};
//# sourceMappingURL=lineForPosition.js.map