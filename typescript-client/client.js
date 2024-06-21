"use strict";
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.usernameToEmail = exports.uint8ArrayToBase64 = exports.base64ToUint8Array = exports.getIdToken = exports.getResumeEndpoints = exports.getResumeUrls = exports.writeS3File = exports.loadS3FileStream = exports.loadS3File = exports.denoS3LightClientSettings = exports.databaseUrlFromResource = exports.setVariable = exports.getVariable = exports.getState = exports.getInternalState = exports.getFlowUserState = exports.setFlowUserState = exports.setState = exports.setInternalState = exports.setResource = exports.getStatePath = exports.resolveDefaultResource = exports.runScriptAsync = exports.task = exports.getResultMaybe = exports.getResult = exports.waitJob = exports.runScript = exports.getRootJobId = exports.getResource = exports.getWorkspace = exports.setClient = exports.SHARED_FOLDER = exports.WorkspaceService = exports.UserService = exports.SettingsService = exports.ScheduleService = exports.ScriptService = exports.VariableService = exports.ResourceService = exports.JobService = exports.GroupService = exports.GranularAclService = exports.FlowService = exports.AuditService = exports.AdminService = void 0;
const index_1 = require("./index");
const index_2 = require("./index");
var index_3 = require("./index");
Object.defineProperty(exports, "AdminService", { enumerable: true, get: function () { return index_3.AdminService; } });
Object.defineProperty(exports, "AuditService", { enumerable: true, get: function () { return index_3.AuditService; } });
Object.defineProperty(exports, "FlowService", { enumerable: true, get: function () { return index_3.FlowService; } });
Object.defineProperty(exports, "GranularAclService", { enumerable: true, get: function () { return index_3.GranularAclService; } });
Object.defineProperty(exports, "GroupService", { enumerable: true, get: function () { return index_3.GroupService; } });
Object.defineProperty(exports, "JobService", { enumerable: true, get: function () { return index_3.JobService; } });
Object.defineProperty(exports, "ResourceService", { enumerable: true, get: function () { return index_3.ResourceService; } });
Object.defineProperty(exports, "VariableService", { enumerable: true, get: function () { return index_3.VariableService; } });
Object.defineProperty(exports, "ScriptService", { enumerable: true, get: function () { return index_3.ScriptService; } });
Object.defineProperty(exports, "ScheduleService", { enumerable: true, get: function () { return index_3.ScheduleService; } });
Object.defineProperty(exports, "SettingsService", { enumerable: true, get: function () { return index_3.SettingsService; } });
Object.defineProperty(exports, "UserService", { enumerable: true, get: function () { return index_3.UserService; } });
Object.defineProperty(exports, "WorkspaceService", { enumerable: true, get: function () { return index_3.WorkspaceService; } });
exports.SHARED_FOLDER = "/shared";
function setClient(token, baseUrl) {
    var _a, _b, _c;
    if (baseUrl === undefined) {
        baseUrl =
            (_b = (_a = getEnv("BASE_INTERNAL_URL")) !== null && _a !== void 0 ? _a : getEnv("BASE_URL")) !== null && _b !== void 0 ? _b : "http://localhost:8000";
    }
    if (token === undefined) {
        token = (_c = getEnv("WM_TOKEN")) !== null && _c !== void 0 ? _c : "no_token";
    }
    index_2.OpenAPI.WITH_CREDENTIALS = true;
    index_2.OpenAPI.TOKEN = token;
    index_2.OpenAPI.BASE = baseUrl + "/api";
}
exports.setClient = setClient;
const getEnv = (key) => {
    var _a, _b, _c;
    if (typeof window === "undefined") {
        // node
        return (_a = process === null || process === void 0 ? void 0 : process.env) === null || _a === void 0 ? void 0 : _a[key];
    }
    // browser
    return (_c = (_b = window === null || window === void 0 ? void 0 : window.process) === null || _b === void 0 ? void 0 : _b.env) === null || _c === void 0 ? void 0 : _c[key];
};
/**
 * Create a client configuration from env variables
 * @returns client configuration
 */
function getWorkspace() {
    var _a;
    return (_a = getEnv("WM_WORKSPACE")) !== null && _a !== void 0 ? _a : "no_workspace";
}
exports.getWorkspace = getWorkspace;
/**
 * Get a resource value by path
 * @param path path of the resource,  default to internal state path
 * @param undefinedIfEmpty if the resource does not exist, return undefined instead of throwing an error
 * @returns resource value
 */
function getResource(path, undefinedIfEmpty) {
    return __awaiter(this, void 0, void 0, function* () {
        const workspace = getWorkspace();
        path = path !== null && path !== void 0 ? path : getStatePath();
        try {
            return yield index_1.ResourceService.getResourceValueInterpolated({
                workspace,
                path,
            });
        }
        catch (e) {
            if (undefinedIfEmpty && e.status === 404) {
                return undefined;
            }
            else {
                throw Error(`Resource not found at ${path} or not visible to you: ${e.body}`);
            }
        }
    });
}
exports.getResource = getResource;
/**
 * Get a resource value by path
 * @param jobId job id to get the root job id from (default to current job)
 * @returns root job id
 */
function getRootJobId(jobId) {
    return __awaiter(this, void 0, void 0, function* () {
        const workspace = getWorkspace();
        jobId = jobId !== null && jobId !== void 0 ? jobId : getEnv("WM_JOB_ID");
        if (jobId === undefined) {
            throw Error("Job ID not set");
        }
        return yield index_1.JobService.getRootJobId({ workspace, id: jobId });
    });
}
exports.getRootJobId = getRootJobId;
function runScript() {
    return __awaiter(this, arguments, void 0, function* (path = null, hash_ = null, args = null, verbose = false) {
        args = args || {};
        if (verbose) {
            console.info(`running \`${path}\` synchronously with args:`, args);
        }
        const jobId = yield runScriptAsync(path, hash_, args);
        return yield waitJob(jobId, verbose);
    });
}
exports.runScript = runScript;
function waitJob(jobId_1) {
    return __awaiter(this, arguments, void 0, function* (jobId, verbose = false) {
        while (true) {
            // Implement your HTTP request logic here to get job result
            const resultRes = yield getResultMaybe(jobId);
            const started = resultRes.started;
            const completed = resultRes.completed;
            const success = resultRes.success;
            if (!started && verbose) {
                console.info(`job ${jobId} has not started yet`);
            }
            if (completed) {
                const result = resultRes.result;
                if (success) {
                    return result;
                }
                else {
                    const error = result.error;
                    throw new Error(`Job ${jobId} was not successful: ${JSON.stringify(error)}`);
                }
            }
            if (verbose) {
                console.info(`sleeping 0.5 seconds for jobId: ${jobId}`);
            }
            yield new Promise((resolve) => setTimeout(resolve, 500));
        }
    });
}
exports.waitJob = waitJob;
function getResult(jobId) {
    return __awaiter(this, void 0, void 0, function* () {
        const workspace = getWorkspace();
        return yield index_1.JobService.getCompletedJobResult({ workspace, id: jobId });
    });
}
exports.getResult = getResult;
function getResultMaybe(jobId) {
    return __awaiter(this, void 0, void 0, function* () {
        const workspace = getWorkspace();
        return yield index_1.JobService.getCompletedJobResultMaybe({ workspace, id: jobId });
    });
}
exports.getResultMaybe = getResultMaybe;
const STRIP_COMMENTS = /(\/\/.*$)|(\/\*[\s\S]*?\*\/)|(\s*=[^,\)]*(('(?:\\'|[^'\r\n])*')|("(?:\\"|[^"\r\n])*"))|(\s*=[^,\)]*))/gm;
const ARGUMENT_NAMES = /([^\s,]+)/g;
function getParamNames(func) {
    const fnStr = func.toString().replace(STRIP_COMMENTS, "");
    let result = fnStr
        .slice(fnStr.indexOf("(") + 1, fnStr.indexOf(")"))
        .match(ARGUMENT_NAMES);
    if (result === null)
        result = [];
    return result;
}
function task(f) {
    return (...y) => __awaiter(this, void 0, void 0, function* () {
        const args = {};
        const paramNames = getParamNames(f);
        y.forEach((x, i) => (args[paramNames[i]] = x));
        let req = yield fetch(`${index_2.OpenAPI.BASE}/w/${getWorkspace()}/jobs/run/workflow_as_code/${getEnv("WM_JOB_ID")}/${f.name}`, {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
                Authorization: `Bearer ${getEnv("WM_TOKEN")}`,
            },
            body: JSON.stringify({ args }),
        });
        let jobId = yield req.text();
        console.log(`Started task ${f.name} as job ${jobId}`);
        let r = yield waitJob(jobId);
        console.log(`Task ${f.name} (${jobId}) completed`);
        return r;
    });
}
exports.task = task;
function runScriptAsync(path_1, hash_1, args_1) {
    return __awaiter(this, arguments, void 0, function* (path, hash_, args, scheduledInSeconds = null) {
        // Create a script job and return its job id.
        if (path && hash_) {
            throw new Error("path and hash_ are mutually exclusive");
        }
        args = args || {};
        const params = {};
        if (scheduledInSeconds) {
            params["scheduled_in_secs"] = scheduledInSeconds;
        }
        let parentJobId = getEnv("WM_JOB_ID");
        if (parentJobId !== undefined) {
            params["parent_job"] = parentJobId;
        }
        let rootJobId = getEnv("WM_ROOT_FLOW_JOB_ID");
        if (rootJobId != undefined && rootJobId != "") {
            params["root_job"] = rootJobId;
        }
        let endpoint;
        if (path) {
            endpoint = `/w/${getWorkspace()}/jobs/run/p/${path}`;
        }
        else if (hash_) {
            endpoint = `/w/${getWorkspace()}/jobs/run/h/${hash_}`;
        }
        else {
            throw new Error("path or hash_ must be provided");
        }
        let url = new URL(index_2.OpenAPI.BASE + endpoint);
        url.search = new URLSearchParams(params).toString();
        return fetch(url, {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
                Authorization: `Bearer ${index_2.OpenAPI.TOKEN}`,
            },
            body: JSON.stringify(args),
        }).then((res) => res.text());
    });
}
exports.runScriptAsync = runScriptAsync;
/**
 * Resolve a resource value in case the default value was picked because the input payload was undefined
 * @param obj resource value or path of the resource under the format `$res:path`
 * @returns resource value
 */
function resolveDefaultResource(obj) {
    return __awaiter(this, void 0, void 0, function* () {
        if (typeof obj === "string" && obj.startsWith("$res:")) {
            return yield getResource(obj.substring(5), true);
        }
        else {
            return obj;
        }
    });
}
exports.resolveDefaultResource = resolveDefaultResource;
function getStatePath() {
    var _a;
    const state_path = (_a = getEnv("WM_STATE_PATH_NEW")) !== null && _a !== void 0 ? _a : getEnv("WM_STATE_PATH");
    if (state_path === undefined) {
        throw Error("State path not set");
    }
    return state_path;
}
exports.getStatePath = getStatePath;
/**
 * Set a resource value by path
 * @param path path of the resource to set, default to state path
 * @param value new value of the resource to set
 * @param initializeToTypeIfNotExist if the resource does not exist, initialize it with this type
 */
function setResource(value, path, initializeToTypeIfNotExist) {
    return __awaiter(this, void 0, void 0, function* () {
        path = path !== null && path !== void 0 ? path : getStatePath();
        const workspace = getWorkspace();
        if (yield index_1.ResourceService.existsResource({ workspace, path })) {
            yield index_1.ResourceService.updateResourceValue({
                workspace,
                path,
                requestBody: { value },
            });
        }
        else if (initializeToTypeIfNotExist) {
            yield index_1.ResourceService.createResource({
                workspace,
                requestBody: { path, value, resource_type: initializeToTypeIfNotExist },
            });
        }
        else {
            throw Error(`Resource at path ${path} does not exist and no type was provided to initialize it`);
        }
    });
}
exports.setResource = setResource;
/**
 * Set the state
 * @param state state to set
 * @deprecated use setState instead
 */
function setInternalState(state) {
    return __awaiter(this, void 0, void 0, function* () {
        yield setResource(state, undefined, "state");
    });
}
exports.setInternalState = setInternalState;
/**
 * Set the state
 * @param state state to set
 */
function setState(state) {
    return __awaiter(this, void 0, void 0, function* () {
        yield setResource(state, undefined, "state");
    });
}
exports.setState = setState;
/**
 * Set a flow user state
 * @param key key of the state
 * @param value value of the state

 */
function setFlowUserState(key, value, errorIfNotPossible) {
    return __awaiter(this, void 0, void 0, function* () {
        if (value === undefined) {
            value = null;
        }
        const workspace = getWorkspace();
        try {
            yield index_1.JobService.setFlowUserState({
                workspace,
                id: yield getRootJobId(),
                key,
                requestBody: value,
            });
        }
        catch (e) {
            if (errorIfNotPossible) {
                throw Error(`Error setting flow user state at ${key}: ${e.body}`);
            }
            else {
                console.error(`Error setting flow user state at ${key}: ${e.body}`);
            }
        }
    });
}
exports.setFlowUserState = setFlowUserState;
/**
 * Get a flow user state
 * @param path path of the variable

 */
function getFlowUserState(key, errorIfNotPossible) {
    return __awaiter(this, void 0, void 0, function* () {
        const workspace = getWorkspace();
        try {
            return yield index_1.JobService.getFlowUserState({
                workspace,
                id: yield getRootJobId(),
                key,
            });
        }
        catch (e) {
            if (errorIfNotPossible) {
                throw Error(`Error setting flow user state at ${key}: ${e.body}`);
            }
            else {
                console.error(`Error setting flow user state at ${key}: ${e.body}`);
            }
        }
    });
}
exports.getFlowUserState = getFlowUserState;
// /**
//  * Set the shared state
//  * @param state state to set
//  */
// export async function setSharedState(
//   state: any,
//   path = "state.json"
// ): Promise<void> {
//   await Deno.writeTextFile(SHARED_FOLDER + "/" + path, JSON.stringify(state));
// }
// /**
//  * Get the shared state
//  * @param state state to set
//  */
// export async function getSharedState(path = "state.json"): Promise<any> {
//   return JSON.parse(await Deno.readTextFile(SHARED_FOLDER + "/" + path));
// }
/**
 * Get the internal state
 * @deprecated use getState instead
 */
function getInternalState() {
    return __awaiter(this, void 0, void 0, function* () {
        return yield getResource(getStatePath(), true);
    });
}
exports.getInternalState = getInternalState;
/**
 * Get the state shared across executions
 */
function getState() {
    return __awaiter(this, void 0, void 0, function* () {
        return yield getResource(getStatePath(), true);
    });
}
exports.getState = getState;
/**
 * Get a variable by path
 * @param path path of the variable
 * @returns variable value
 */
function getVariable(path) {
    return __awaiter(this, void 0, void 0, function* () {
        const workspace = getWorkspace();
        try {
            return yield index_1.VariableService.getVariableValue({ workspace, path });
        }
        catch (e) {
            throw Error(`Variable not found at ${path} or not visible to you: ${e.body}`);
        }
    });
}
exports.getVariable = getVariable;
/**
 * Set a variable by path, create if not exist
 * @param path path of the variable
 * @param value value of the variable
 * @param isSecretIfNotExist if the variable does not exist, create it as secret or not (default: false)
 * @param descriptionIfNotExist if the variable does not exist, create it with this description (default: "")
 */
function setVariable(path, value, isSecretIfNotExist, descriptionIfNotExist) {
    return __awaiter(this, void 0, void 0, function* () {
        const workspace = getWorkspace();
        if (yield index_1.VariableService.existsVariable({ workspace, path })) {
            yield index_1.VariableService.updateVariable({
                workspace,
                path,
                requestBody: { value },
            });
        }
        else {
            yield index_1.VariableService.createVariable({
                workspace,
                requestBody: {
                    path,
                    value,
                    is_secret: isSecretIfNotExist !== null && isSecretIfNotExist !== void 0 ? isSecretIfNotExist : false,
                    description: descriptionIfNotExist !== null && descriptionIfNotExist !== void 0 ? descriptionIfNotExist : "",
                },
            });
        }
    });
}
exports.setVariable = setVariable;
function databaseUrlFromResource(path) {
    return __awaiter(this, void 0, void 0, function* () {
        const resource = yield getResource(path);
        return `postgresql://${resource.user}:${resource.password}@${resource.host}:${resource.port}/${resource.dbname}?sslmode=${resource.sslmode}`;
    });
}
exports.databaseUrlFromResource = databaseUrlFromResource;
// TODO(gb): need to investigate more how Polars and DuckDB work in TS
// export async function polarsConnectionSettings(s3_resource_path: string | undefined): Promise<any> {
//   const workspace = getWorkspace();
//   return await HelpersService.polarsConnectionSettingsV2({
//     workspace: workspace,
// 		requestBody: {
// 			s3_resource_path: s3_resource_path
// 		}
//   });
// }
// export async function duckdbConnectionSettings(s3_resource_path: string | undefined): Promise<any> {
//   const workspace = getWorkspace();
//   return await HelpersService.duckdbConnectionSettingsV2({
//     workspace: workspace,
// 		requestBody: {
// 			s3_resource_path: s3_resource_path
// 		}
//   });
// }
function denoS3LightClientSettings(s3_resource_path) {
    return __awaiter(this, void 0, void 0, function* () {
        const workspace = getWorkspace();
        const s3Resource = yield index_1.HelpersService.s3ResourceInfo({
            workspace: workspace,
            requestBody: {
                s3_resource_path: s3_resource_path,
            },
        });
        let settings = Object.assign({}, s3Resource);
        return settings;
    });
}
exports.denoS3LightClientSettings = denoS3LightClientSettings;
/**
 * Load the content of a file stored in S3. If the s3ResourcePath is undefined, it will default to the workspace S3 resource.
 *
 * ```typescript
 * let fileContent = await wmill.loadS3FileContent(inputFile)
 * // if the file is a raw text file, it can be decoded and printed directly:
 * const text = new TextDecoder().decode(fileContentStream)
 * console.log(text);
 * ```
 */
function loadS3File(s3object_1) {
    return __awaiter(this, arguments, void 0, function* (s3object, s3ResourcePath = undefined) {
        const fileContentBlob = yield loadS3FileStream(s3object, s3ResourcePath);
        if (fileContentBlob === undefined) {
            return undefined;
        }
        // we read the stream until completion and put the content in an Uint8Array
        const reader = fileContentBlob.stream().getReader();
        const chunks = [];
        while (true) {
            const { value: chunk, done } = yield reader.read();
            if (done) {
                break;
            }
            chunks.push(chunk);
        }
        let fileContentLength = 0;
        chunks.forEach((item) => {
            fileContentLength += item.length;
        });
        let fileContent = new Uint8Array(fileContentLength);
        let offset = 0;
        chunks.forEach((chunk) => {
            fileContent.set(chunk, offset);
            offset += chunk.length;
        });
        return fileContent;
    });
}
exports.loadS3File = loadS3File;
/**
 * Load the content of a file stored in S3 as a stream. If the s3ResourcePath is undefined, it will default to the workspace S3 resource.
 *
 * ```typescript
 * let fileContentBlob = await wmill.loadS3FileStream(inputFile)
 * // if the content is plain text, the blob can be read directly:
 * console.log(await fileContentBlob.text());
 * ```
 */
function loadS3FileStream(s3object_1) {
    return __awaiter(this, arguments, void 0, function* (s3object, s3ResourcePath = undefined) {
        let params = {};
        params["file_key"] = s3object.s3;
        if (s3ResourcePath !== undefined) {
            params["s3_resource_path"] = s3ResourcePath;
        }
        if (s3object.storage !== undefined) {
            params["storage"] = s3object.storage;
        }
        const queryParams = new URLSearchParams(params);
        // We use raw fetch here b/c OpenAPI generated client doesn't handle Blobs nicely
        const fileContentBlob = yield fetch(`${index_2.OpenAPI.BASE}/w/${getWorkspace()}/job_helpers/download_s3_file?${queryParams}`, {
            method: "GET",
            headers: {
                Authorization: `Bearer ${index_2.OpenAPI.TOKEN}`,
            },
        });
        return fileContentBlob.blob();
    });
}
exports.loadS3FileStream = loadS3FileStream;
/**
 * Persist a file to the S3 bucket. If the s3ResourcePath is undefined, it will default to the workspace S3 resource.
 *
 * ```typescript
 * const s3object = await writeS3File(s3Object, "Hello Windmill!")
 * const fileContentAsUtf8Str = (await s3object.toArray()).toString('utf-8')
 * console.log(fileContentAsUtf8Str)
 * ```
 */
function writeS3File(s3object_1, fileContent_1) {
    return __awaiter(this, arguments, void 0, function* (s3object, fileContent, s3ResourcePath = undefined) {
        let fileContentBlob;
        if (typeof fileContent === "string") {
            fileContentBlob = new Blob([fileContent], {
                type: "text/plain",
            });
        }
        else {
            fileContentBlob = fileContent;
        }
        const response = yield index_1.HelpersService.fileUpload({
            workspace: getWorkspace(),
            fileKey: s3object === null || s3object === void 0 ? void 0 : s3object.s3,
            fileExtension: undefined,
            s3ResourcePath: s3ResourcePath,
            requestBody: fileContentBlob,
            storage: s3object === null || s3object === void 0 ? void 0 : s3object.storage,
        });
        return {
            s3: response.file_key,
        };
    });
}
exports.writeS3File = writeS3File;
/**
 * Get URLs needed for resuming a flow after this step
 * @param approver approver name
 * @returns approval page UI URL, resume and cancel API URLs for resuming the flow
 */
function getResumeUrls(approver) {
    return __awaiter(this, void 0, void 0, function* () {
        var _a;
        const nonce = Math.floor(Math.random() * 4294967295);
        const workspace = getWorkspace();
        return yield index_1.JobService.getResumeUrls({
            workspace,
            resumeId: nonce,
            approver,
            id: (_a = getEnv("WM_JOB_ID")) !== null && _a !== void 0 ? _a : "NO_JOB_ID",
        });
    });
}
exports.getResumeUrls = getResumeUrls;
/**
 * @deprecated use getResumeUrls instead
 */
function getResumeEndpoints(approver) {
    return getResumeUrls(approver);
}
exports.getResumeEndpoints = getResumeEndpoints;
/**
 * Get an OIDC jwt token for auth to external services (e.g: Vault, AWS) (ee only)
 * @param audience audience of the token
 * @returns jwt token
 */
function getIdToken(audience) {
    return __awaiter(this, void 0, void 0, function* () {
        const workspace = getWorkspace();
        return yield index_1.OidcService.getOidcToken({
            workspace,
            audience,
        });
    });
}
exports.getIdToken = getIdToken;
function base64ToUint8Array(data) {
    return Uint8Array.from(atob(data), (c) => c.charCodeAt(0));
}
exports.base64ToUint8Array = base64ToUint8Array;
function uint8ArrayToBase64(arrayBuffer) {
    let base64 = "";
    const encodings = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    const bytes = new Uint8Array(arrayBuffer);
    const byteLength = bytes.byteLength;
    const byteRemainder = byteLength % 3;
    const mainLength = byteLength - byteRemainder;
    let a, b, c, d;
    let chunk;
    // Main loop deals with bytes in chunks of 3
    for (let i = 0; i < mainLength; i = i + 3) {
        // Combine the three bytes into a single integer
        chunk = (bytes[i] << 16) | (bytes[i + 1] << 8) | bytes[i + 2];
        // Use bitmasks to extract 6-bit segments from the triplet
        a = (chunk & 16515072) >> 18; // 16515072 = (2^6 - 1) << 18
        b = (chunk & 258048) >> 12; // 258048   = (2^6 - 1) << 12
        c = (chunk & 4032) >> 6; // 4032     = (2^6 - 1) << 6
        d = chunk & 63; // 63       = 2^6 - 1
        // Convert the raw binary segments to the appropriate ASCII encoding
        base64 += encodings[a] + encodings[b] + encodings[c] + encodings[d];
    }
    // Deal with the remaining bytes and padding
    if (byteRemainder == 1) {
        chunk = bytes[mainLength];
        a = (chunk & 252) >> 2; // 252 = (2^6 - 1) << 2
        // Set the 4 least significant bits to zero
        b = (chunk & 3) << 4; // 3   = 2^2 - 1
        base64 += encodings[a] + encodings[b] + "==";
    }
    else if (byteRemainder == 2) {
        chunk = (bytes[mainLength] << 8) | bytes[mainLength + 1];
        a = (chunk & 64512) >> 10; // 64512 = (2^6 - 1) << 10
        b = (chunk & 1008) >> 4; // 1008  = (2^6 - 1) << 4
        // Set the 2 least significant bits to zero
        c = (chunk & 15) << 2; // 15    = 2^4 - 1
        base64 += encodings[a] + encodings[b] + encodings[c] + "=";
    }
    return base64;
}
exports.uint8ArrayToBase64 = uint8ArrayToBase64;
/**
 * Get email from workspace username
 * This method is particularly useful for apps that require the email address of the viewer.
 * Indeed, in the viewer context, WM_USERNAME is set to the username of the viewer but WM_EMAIL is set to the email of the creator of the app.
 * @param username
 * @returns email address
 */
function usernameToEmail(username) {
    return __awaiter(this, void 0, void 0, function* () {
        const workspace = getWorkspace();
        return yield index_1.UserService.usernameToEmail({ username, workspace });
    });
}
exports.usernameToEmail = usernameToEmail;
