'use strict';
class YAMLException {
    constructor(reason, mark = null, isWarning = false) {
        this.name = 'YAMLException';
        this.reason = reason;
        this.mark = mark;
        this.message = this.toString(false);
        this.isWarning = isWarning;
    }
    static isInstance(instance) {
        if (instance != null && instance.getClassIdentifier
            && typeof (instance.getClassIdentifier) == "function") {
            for (let currentIdentifier of instance.getClassIdentifier()) {
                if (currentIdentifier == YAMLException.CLASS_IDENTIFIER)
                    return true;
            }
        }
        return false;
    }
    getClassIdentifier() {
        var superIdentifiers = [];
        return superIdentifiers.concat(YAMLException.CLASS_IDENTIFIER);
    }
    toString(compact = false) {
        var result;
        result = 'JS-YAML: ' + (this.reason || '(unknown reason)');
        if (!compact && this.mark) {
            result += ' ' + this.mark.toString();
        }
        return result;
    }
}
YAMLException.CLASS_IDENTIFIER = "yaml-ast-parser.YAMLException";
module.exports = YAMLException;
//# sourceMappingURL=exception.js.map