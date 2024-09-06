"use strict";
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
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || function (mod) {
    if (mod && mod.__esModule) return mod;
    var result = {};
    if (mod != null) for (var k in mod) if (k !== "default" && Object.prototype.hasOwnProperty.call(mod, k)) __createBinding(result, mod, k);
    __setModuleDefault(result, mod);
    return result;
};
var __exportStar = (this && this.__exportStar) || function(m, exports) {
    for (var p in m) if (p !== "default" && !Object.prototype.hasOwnProperty.call(exports, p)) __createBinding(exports, m, p);
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.version = exports.resources = exports.ppid = exports.pid = exports.permissions = exports.noColor = exports.metrics = exports.mainModule = exports.errors = exports.env = exports.customInspect = exports.build = void 0;
var build_js_1 = require("./variables/build.js");
Object.defineProperty(exports, "build", { enumerable: true, get: function () { return build_js_1.build; } });
var customInspect_js_1 = require("./variables/customInspect.js");
Object.defineProperty(exports, "customInspect", { enumerable: true, get: function () { return customInspect_js_1.customInspect; } });
var env_js_1 = require("./variables/env.js");
Object.defineProperty(exports, "env", { enumerable: true, get: function () { return env_js_1.env; } });
exports.errors = __importStar(require("./variables/errors.js"));
var mainModule_js_1 = require("./variables/mainModule.js");
Object.defineProperty(exports, "mainModule", { enumerable: true, get: function () { return mainModule_js_1.mainModule; } });
var metrics_js_1 = require("./variables/metrics.js");
Object.defineProperty(exports, "metrics", { enumerable: true, get: function () { return metrics_js_1.metrics; } });
var noColor_js_1 = require("./variables/noColor.js");
Object.defineProperty(exports, "noColor", { enumerable: true, get: function () { return noColor_js_1.noColor; } });
var permissions_js_1 = require("./variables/permissions.js");
Object.defineProperty(exports, "permissions", { enumerable: true, get: function () { return permissions_js_1.permissions; } });
var pid_js_1 = require("./variables/pid.js");
Object.defineProperty(exports, "pid", { enumerable: true, get: function () { return pid_js_1.pid; } });
var ppid_js_1 = require("./variables/ppid.js");
Object.defineProperty(exports, "ppid", { enumerable: true, get: function () { return ppid_js_1.ppid; } });
var resources_js_1 = require("./variables/resources.js");
Object.defineProperty(exports, "resources", { enumerable: true, get: function () { return resources_js_1.resources; } });
__exportStar(require("./variables/std.js"), exports);
var version_js_1 = require("./variables/version.js");
Object.defineProperty(exports, "version", { enumerable: true, get: function () { return version_js_1.version; } });
