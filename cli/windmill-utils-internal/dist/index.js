"use strict";
/**
 * @fileoverview Main entry point for windmill-utils-internal package
 *
 * This module provides utilities for handling Windmill flows, scripts, and schemas:
 * - Inline script extraction and replacement
 * - Path utilities for different programming languages
 * - Schema parsing and conversion utilities
 * - Cross-platform path constants
 */
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __exportStar = (this && this.__exportStar) || function(m, exports) {
    for (var p in m) if (p !== "default" && !Object.prototype.hasOwnProperty.call(exports, p)) __createBinding(exports, m, p);
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.DELIMITER = exports.SEP = void 0;
__exportStar(require("./inline-scripts"), exports);
__exportStar(require("./path-utils"), exports);
__exportStar(require("./parse"), exports);
var constants_1 = require("./constants");
Object.defineProperty(exports, "SEP", { enumerable: true, get: function () { return constants_1.SEP; } });
Object.defineProperty(exports, "DELIMITER", { enumerable: true, get: function () { return constants_1.DELIMITER; } });
