"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const yaml_ast_parser_1 = require("@stoplight/yaml-ast-parser");
exports.safeStringify = (value, options) => typeof value === 'string' ? value : yaml_ast_parser_1.safeDump(value, options);
//# sourceMappingURL=safeStringify.js.map