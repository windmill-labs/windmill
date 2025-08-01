"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.TsJestDiagnosticCodes = void 0;
exports.interpolate = interpolate;
/**
 * @internal
 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
function interpolate(msg, vars = {}) {
    // eslint-disable-next-line no-useless-escape
    return msg.replace(/\{\{([^\}]+)\}\}/g, (_, key) => (key in vars ? vars[key] : _));
}
exports.TsJestDiagnosticCodes = {
    Generic: 151000,
    ConfigModuleOption: 151001,
};
