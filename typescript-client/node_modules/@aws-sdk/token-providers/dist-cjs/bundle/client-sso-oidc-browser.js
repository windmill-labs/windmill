"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.UnsupportedGrantTypeException = exports.UnauthorizedClientException = exports.SlowDownException = exports.SSOOIDCClient = exports.InvalidScopeException = exports.InvalidRequestException = exports.InvalidClientException = exports.InternalServerException = exports.ExpiredTokenException = exports.CreateTokenCommand = exports.AuthorizationPendingException = exports.AccessDeniedException = void 0;
const middleware_host_header_1 = require("@aws-sdk/middleware-host-header");
const middleware_logger_1 = require("@aws-sdk/middleware-logger");
const middleware_recursion_detection_1 = require("@aws-sdk/middleware-recursion-detection");
const middleware_user_agent_1 = require("@aws-sdk/middleware-user-agent");
const config_resolver_1 = require("@smithy/config-resolver");
const middleware_content_length_1 = require("@smithy/middleware-content-length");
const middleware_endpoint_1 = require("@smithy/middleware-endpoint");
const middleware_retry_1 = require("@smithy/middleware-retry");
const smithy_client_1 = require("@smithy/smithy-client");
var resolveClientEndpointParameters = (options) => {
    var _a, _b;
    return {
        ...options,
        useDualstackEndpoint: (_a = options.useDualstackEndpoint) !== null && _a !== void 0 ? _a : false,
        useFipsEndpoint: (_b = options.useFipsEndpoint) !== null && _b !== void 0 ? _b : false,
        defaultSigningName: "awsssooidc",
    };
};
var package_default = { version: "3.429.0" };
const sha256_browser_1 = require("@aws-crypto/sha256-browser");
const util_user_agent_browser_1 = require("@aws-sdk/util-user-agent-browser");
const config_resolver_2 = require("@smithy/config-resolver");
const fetch_http_handler_1 = require("@smithy/fetch-http-handler");
const invalid_dependency_1 = require("@smithy/invalid-dependency");
const util_body_length_browser_1 = require("@smithy/util-body-length-browser");
const util_retry_1 = require("@smithy/util-retry");
const smithy_client_2 = require("@smithy/smithy-client");
const url_parser_1 = require("@smithy/url-parser");
const util_base64_1 = require("@smithy/util-base64");
const util_utf8_1 = require("@smithy/util-utf8");
const util_endpoints_1 = require("@aws-sdk/util-endpoints");
var s = "required";
var t = "fn";
var u = "argv";
var v = "ref";
var a = "isSet";
var b = "tree";
var c = "error";
var d = "endpoint";
var e = "PartitionResult";
var f = "getAttr";
var g = { [s]: false, type: "String" };
var h = { [s]: true, default: false, type: "Boolean" };
var i = { [v]: "Endpoint" };
var j = { [t]: "booleanEquals", [u]: [{ [v]: "UseFIPS" }, true] };
var k = { [t]: "booleanEquals", [u]: [{ [v]: "UseDualStack" }, true] };
var l = {};
var m = { [t]: "booleanEquals", [u]: [true, { [t]: f, [u]: [{ [v]: e }, "supportsFIPS"] }] };
var n = { [v]: e };
var o = { [t]: "booleanEquals", [u]: [true, { [t]: f, [u]: [n, "supportsDualStack"] }] };
var p = [j];
var q = [k];
var r = [{ [v]: "Region" }];
var _data = {
    version: "1.0",
    parameters: { Region: g, UseDualStack: h, UseFIPS: h, Endpoint: g },
    rules: [
        {
            conditions: [{ [t]: a, [u]: [i] }],
            type: b,
            rules: [
                { conditions: p, error: "Invalid Configuration: FIPS and custom endpoint are not supported", type: c },
                { conditions: q, error: "Invalid Configuration: Dualstack and custom endpoint are not supported", type: c },
                { endpoint: { url: i, properties: l, headers: l }, type: d },
            ],
        },
        {
            conditions: [{ [t]: a, [u]: r }],
            type: b,
            rules: [
                {
                    conditions: [{ [t]: "aws.partition", [u]: r, assign: e }],
                    type: b,
                    rules: [
                        {
                            conditions: [j, k],
                            type: b,
                            rules: [
                                {
                                    conditions: [m, o],
                                    type: b,
                                    rules: [
                                        {
                                            endpoint: {
                                                url: "https://oidc-fips.{Region}.{PartitionResult#dualStackDnsSuffix}",
                                                properties: l,
                                                headers: l,
                                            },
                                            type: d,
                                        },
                                    ],
                                },
                                { error: "FIPS and DualStack are enabled, but this partition does not support one or both", type: c },
                            ],
                        },
                        {
                            conditions: p,
                            type: b,
                            rules: [
                                {
                                    conditions: [m],
                                    type: b,
                                    rules: [
                                        {
                                            conditions: [{ [t]: "stringEquals", [u]: ["aws-us-gov", { [t]: f, [u]: [n, "name"] }] }],
                                            endpoint: { url: "https://oidc.{Region}.amazonaws.com", properties: l, headers: l },
                                            type: d,
                                        },
                                        {
                                            endpoint: {
                                                url: "https://oidc-fips.{Region}.{PartitionResult#dnsSuffix}",
                                                properties: l,
                                                headers: l,
                                            },
                                            type: d,
                                        },
                                    ],
                                },
                                { error: "FIPS is enabled but this partition does not support FIPS", type: c },
                            ],
                        },
                        {
                            conditions: q,
                            type: b,
                            rules: [
                                {
                                    conditions: [o],
                                    type: b,
                                    rules: [
                                        {
                                            endpoint: {
                                                url: "https://oidc.{Region}.{PartitionResult#dualStackDnsSuffix}",
                                                properties: l,
                                                headers: l,
                                            },
                                            type: d,
                                        },
                                    ],
                                },
                                { error: "DualStack is enabled but this partition does not support DualStack", type: c },
                            ],
                        },
                        {
                            endpoint: { url: "https://oidc.{Region}.{PartitionResult#dnsSuffix}", properties: l, headers: l },
                            type: d,
                        },
                    ],
                },
            ],
        },
        { error: "Invalid Configuration: Missing Region", type: c },
    ],
};
var ruleSet = _data;
var defaultEndpointResolver = (endpointParams, context = {}) => {
    return (0, util_endpoints_1.resolveEndpoint)(ruleSet, {
        endpointParams,
        logger: context.logger,
    });
};
var getRuntimeConfig = (config) => {
    var _a, _b, _c, _d, _e, _f, _g, _h, _j, _k;
    return ({
        apiVersion: "2019-06-10",
        base64Decoder: (_a = config === null || config === void 0 ? void 0 : config.base64Decoder) !== null && _a !== void 0 ? _a : util_base64_1.fromBase64,
        base64Encoder: (_b = config === null || config === void 0 ? void 0 : config.base64Encoder) !== null && _b !== void 0 ? _b : util_base64_1.toBase64,
        disableHostPrefix: (_c = config === null || config === void 0 ? void 0 : config.disableHostPrefix) !== null && _c !== void 0 ? _c : false,
        endpointProvider: (_d = config === null || config === void 0 ? void 0 : config.endpointProvider) !== null && _d !== void 0 ? _d : defaultEndpointResolver,
        extensions: (_e = config === null || config === void 0 ? void 0 : config.extensions) !== null && _e !== void 0 ? _e : [],
        logger: (_f = config === null || config === void 0 ? void 0 : config.logger) !== null && _f !== void 0 ? _f : new smithy_client_2.NoOpLogger(),
        serviceId: (_g = config === null || config === void 0 ? void 0 : config.serviceId) !== null && _g !== void 0 ? _g : "SSO OIDC",
        urlParser: (_h = config === null || config === void 0 ? void 0 : config.urlParser) !== null && _h !== void 0 ? _h : url_parser_1.parseUrl,
        utf8Decoder: (_j = config === null || config === void 0 ? void 0 : config.utf8Decoder) !== null && _j !== void 0 ? _j : util_utf8_1.fromUtf8,
        utf8Encoder: (_k = config === null || config === void 0 ? void 0 : config.utf8Encoder) !== null && _k !== void 0 ? _k : util_utf8_1.toUtf8,
    });
};
const smithy_client_3 = require("@smithy/smithy-client");
const util_defaults_mode_browser_1 = require("@smithy/util-defaults-mode-browser");
var getRuntimeConfig2 = (config) => {
    var _a, _b, _c, _d, _e, _f, _g, _h, _j, _k;
    const defaultsMode = (0, util_defaults_mode_browser_1.resolveDefaultsModeConfig)(config);
    const defaultConfigProvider = () => defaultsMode().then(smithy_client_3.loadConfigsForDefaultMode);
    const clientSharedValues = getRuntimeConfig(config);
    return {
        ...clientSharedValues,
        ...config,
        runtime: "browser",
        defaultsMode,
        bodyLengthChecker: (_a = config === null || config === void 0 ? void 0 : config.bodyLengthChecker) !== null && _a !== void 0 ? _a : util_body_length_browser_1.calculateBodyLength,
        defaultUserAgentProvider: (_b = config === null || config === void 0 ? void 0 : config.defaultUserAgentProvider) !== null && _b !== void 0 ? _b : (0, util_user_agent_browser_1.defaultUserAgent)({ serviceId: clientSharedValues.serviceId, clientVersion: package_default.version }),
        maxAttempts: (_c = config === null || config === void 0 ? void 0 : config.maxAttempts) !== null && _c !== void 0 ? _c : util_retry_1.DEFAULT_MAX_ATTEMPTS,
        region: (_d = config === null || config === void 0 ? void 0 : config.region) !== null && _d !== void 0 ? _d : (0, invalid_dependency_1.invalidProvider)("Region is missing"),
        requestHandler: (_e = config === null || config === void 0 ? void 0 : config.requestHandler) !== null && _e !== void 0 ? _e : new fetch_http_handler_1.FetchHttpHandler(defaultConfigProvider),
        retryMode: (_f = config === null || config === void 0 ? void 0 : config.retryMode) !== null && _f !== void 0 ? _f : (async () => (await defaultConfigProvider()).retryMode || util_retry_1.DEFAULT_RETRY_MODE),
        sha256: (_g = config === null || config === void 0 ? void 0 : config.sha256) !== null && _g !== void 0 ? _g : sha256_browser_1.Sha256,
        streamCollector: (_h = config === null || config === void 0 ? void 0 : config.streamCollector) !== null && _h !== void 0 ? _h : fetch_http_handler_1.streamCollector,
        useDualstackEndpoint: (_j = config === null || config === void 0 ? void 0 : config.useDualstackEndpoint) !== null && _j !== void 0 ? _j : (() => Promise.resolve(config_resolver_2.DEFAULT_USE_DUALSTACK_ENDPOINT)),
        useFipsEndpoint: (_k = config === null || config === void 0 ? void 0 : config.useFipsEndpoint) !== null && _k !== void 0 ? _k : (() => Promise.resolve(config_resolver_2.DEFAULT_USE_FIPS_ENDPOINT)),
    };
};
const region_config_resolver_1 = require("@aws-sdk/region-config-resolver");
const protocol_http_1 = require("@smithy/protocol-http");
const smithy_client_4 = require("@smithy/smithy-client");
var asPartial = (t2) => t2;
var resolveRuntimeExtensions = (runtimeConfig, extensions) => {
    const extensionConfiguration = {
        ...asPartial((0, region_config_resolver_1.getAwsRegionExtensionConfiguration)(runtimeConfig)),
        ...asPartial((0, smithy_client_4.getDefaultExtensionConfiguration)(runtimeConfig)),
        ...asPartial((0, protocol_http_1.getHttpHandlerExtensionConfiguration)(runtimeConfig)),
    };
    extensions.forEach((extension) => extension.configure(extensionConfiguration));
    return {
        ...runtimeConfig,
        ...(0, region_config_resolver_1.resolveAwsRegionExtensionConfiguration)(extensionConfiguration),
        ...(0, smithy_client_4.resolveDefaultRuntimeConfig)(extensionConfiguration),
        ...(0, protocol_http_1.resolveHttpHandlerRuntimeConfig)(extensionConfiguration),
    };
};
var SSOOIDCClient = class extends smithy_client_1.Client {
    constructor(...[configuration]) {
        const _config_0 = getRuntimeConfig2(configuration || {});
        const _config_1 = resolveClientEndpointParameters(_config_0);
        const _config_2 = (0, config_resolver_1.resolveRegionConfig)(_config_1);
        const _config_3 = (0, middleware_endpoint_1.resolveEndpointConfig)(_config_2);
        const _config_4 = (0, middleware_retry_1.resolveRetryConfig)(_config_3);
        const _config_5 = (0, middleware_host_header_1.resolveHostHeaderConfig)(_config_4);
        const _config_6 = (0, middleware_user_agent_1.resolveUserAgentConfig)(_config_5);
        const _config_7 = resolveRuntimeExtensions(_config_6, (configuration === null || configuration === void 0 ? void 0 : configuration.extensions) || []);
        super(_config_7);
        this.config = _config_7;
        this.middlewareStack.use((0, middleware_retry_1.getRetryPlugin)(this.config));
        this.middlewareStack.use((0, middleware_content_length_1.getContentLengthPlugin)(this.config));
        this.middlewareStack.use((0, middleware_host_header_1.getHostHeaderPlugin)(this.config));
        this.middlewareStack.use((0, middleware_logger_1.getLoggerPlugin)(this.config));
        this.middlewareStack.use((0, middleware_recursion_detection_1.getRecursionDetectionPlugin)(this.config));
        this.middlewareStack.use((0, middleware_user_agent_1.getUserAgentPlugin)(this.config));
    }
    destroy() {
        super.destroy();
    }
};
exports.SSOOIDCClient = SSOOIDCClient;
const smithy_client_5 = require("@smithy/smithy-client");
const middleware_endpoint_2 = require("@smithy/middleware-endpoint");
const middleware_serde_1 = require("@smithy/middleware-serde");
const smithy_client_6 = require("@smithy/smithy-client");
const types_1 = require("@smithy/types");
const protocol_http_2 = require("@smithy/protocol-http");
const smithy_client_7 = require("@smithy/smithy-client");
const smithy_client_8 = require("@smithy/smithy-client");
var SSOOIDCServiceException = class _SSOOIDCServiceException extends smithy_client_8.ServiceException {
    constructor(options) {
        super(options);
        Object.setPrototypeOf(this, _SSOOIDCServiceException.prototype);
    }
};
var AccessDeniedException = class _AccessDeniedException extends SSOOIDCServiceException {
    constructor(opts) {
        super({
            name: "AccessDeniedException",
            $fault: "client",
            ...opts,
        });
        this.name = "AccessDeniedException";
        this.$fault = "client";
        Object.setPrototypeOf(this, _AccessDeniedException.prototype);
        this.error = opts.error;
        this.error_description = opts.error_description;
    }
};
exports.AccessDeniedException = AccessDeniedException;
var AuthorizationPendingException = class _AuthorizationPendingException extends SSOOIDCServiceException {
    constructor(opts) {
        super({
            name: "AuthorizationPendingException",
            $fault: "client",
            ...opts,
        });
        this.name = "AuthorizationPendingException";
        this.$fault = "client";
        Object.setPrototypeOf(this, _AuthorizationPendingException.prototype);
        this.error = opts.error;
        this.error_description = opts.error_description;
    }
};
exports.AuthorizationPendingException = AuthorizationPendingException;
var ExpiredTokenException = class _ExpiredTokenException extends SSOOIDCServiceException {
    constructor(opts) {
        super({
            name: "ExpiredTokenException",
            $fault: "client",
            ...opts,
        });
        this.name = "ExpiredTokenException";
        this.$fault = "client";
        Object.setPrototypeOf(this, _ExpiredTokenException.prototype);
        this.error = opts.error;
        this.error_description = opts.error_description;
    }
};
exports.ExpiredTokenException = ExpiredTokenException;
var InternalServerException = class _InternalServerException extends SSOOIDCServiceException {
    constructor(opts) {
        super({
            name: "InternalServerException",
            $fault: "server",
            ...opts,
        });
        this.name = "InternalServerException";
        this.$fault = "server";
        Object.setPrototypeOf(this, _InternalServerException.prototype);
        this.error = opts.error;
        this.error_description = opts.error_description;
    }
};
exports.InternalServerException = InternalServerException;
var InvalidClientException = class _InvalidClientException extends SSOOIDCServiceException {
    constructor(opts) {
        super({
            name: "InvalidClientException",
            $fault: "client",
            ...opts,
        });
        this.name = "InvalidClientException";
        this.$fault = "client";
        Object.setPrototypeOf(this, _InvalidClientException.prototype);
        this.error = opts.error;
        this.error_description = opts.error_description;
    }
};
exports.InvalidClientException = InvalidClientException;
var InvalidGrantException = class _InvalidGrantException extends SSOOIDCServiceException {
    constructor(opts) {
        super({
            name: "InvalidGrantException",
            $fault: "client",
            ...opts,
        });
        this.name = "InvalidGrantException";
        this.$fault = "client";
        Object.setPrototypeOf(this, _InvalidGrantException.prototype);
        this.error = opts.error;
        this.error_description = opts.error_description;
    }
};
var InvalidRequestException = class _InvalidRequestException extends SSOOIDCServiceException {
    constructor(opts) {
        super({
            name: "InvalidRequestException",
            $fault: "client",
            ...opts,
        });
        this.name = "InvalidRequestException";
        this.$fault = "client";
        Object.setPrototypeOf(this, _InvalidRequestException.prototype);
        this.error = opts.error;
        this.error_description = opts.error_description;
    }
};
exports.InvalidRequestException = InvalidRequestException;
var InvalidScopeException = class _InvalidScopeException extends SSOOIDCServiceException {
    constructor(opts) {
        super({
            name: "InvalidScopeException",
            $fault: "client",
            ...opts,
        });
        this.name = "InvalidScopeException";
        this.$fault = "client";
        Object.setPrototypeOf(this, _InvalidScopeException.prototype);
        this.error = opts.error;
        this.error_description = opts.error_description;
    }
};
exports.InvalidScopeException = InvalidScopeException;
var SlowDownException = class _SlowDownException extends SSOOIDCServiceException {
    constructor(opts) {
        super({
            name: "SlowDownException",
            $fault: "client",
            ...opts,
        });
        this.name = "SlowDownException";
        this.$fault = "client";
        Object.setPrototypeOf(this, _SlowDownException.prototype);
        this.error = opts.error;
        this.error_description = opts.error_description;
    }
};
exports.SlowDownException = SlowDownException;
var UnauthorizedClientException = class _UnauthorizedClientException extends SSOOIDCServiceException {
    constructor(opts) {
        super({
            name: "UnauthorizedClientException",
            $fault: "client",
            ...opts,
        });
        this.name = "UnauthorizedClientException";
        this.$fault = "client";
        Object.setPrototypeOf(this, _UnauthorizedClientException.prototype);
        this.error = opts.error;
        this.error_description = opts.error_description;
    }
};
exports.UnauthorizedClientException = UnauthorizedClientException;
var UnsupportedGrantTypeException = class _UnsupportedGrantTypeException extends SSOOIDCServiceException {
    constructor(opts) {
        super({
            name: "UnsupportedGrantTypeException",
            $fault: "client",
            ...opts,
        });
        this.name = "UnsupportedGrantTypeException";
        this.$fault = "client";
        Object.setPrototypeOf(this, _UnsupportedGrantTypeException.prototype);
        this.error = opts.error;
        this.error_description = opts.error_description;
    }
};
exports.UnsupportedGrantTypeException = UnsupportedGrantTypeException;
var InvalidClientMetadataException = class _InvalidClientMetadataException extends SSOOIDCServiceException {
    constructor(opts) {
        super({
            name: "InvalidClientMetadataException",
            $fault: "client",
            ...opts,
        });
        this.name = "InvalidClientMetadataException";
        this.$fault = "client";
        Object.setPrototypeOf(this, _InvalidClientMetadataException.prototype);
        this.error = opts.error;
        this.error_description = opts.error_description;
    }
};
var se_CreateTokenCommand = async (input, context) => {
    const { hostname, protocol = "https", port, path: basePath } = await context.endpoint();
    const headers = {
        "content-type": "application/json",
    };
    const resolvedPath = `${(basePath === null || basePath === void 0 ? void 0 : basePath.endsWith("/")) ? basePath.slice(0, -1) : basePath || ""}/token`;
    let body;
    body = JSON.stringify((0, smithy_client_7.take)(input, {
        clientId: [],
        clientSecret: [],
        code: [],
        deviceCode: [],
        grantType: [],
        redirectUri: [],
        refreshToken: [],
        scope: (_) => (0, smithy_client_7._json)(_),
    }));
    return new protocol_http_2.HttpRequest({
        protocol,
        hostname,
        port,
        method: "POST",
        headers,
        path: resolvedPath,
        body,
    });
};
var se_RegisterClientCommand = async (input, context) => {
    const { hostname, protocol = "https", port, path: basePath } = await context.endpoint();
    const headers = {
        "content-type": "application/json",
    };
    const resolvedPath = `${(basePath === null || basePath === void 0 ? void 0 : basePath.endsWith("/")) ? basePath.slice(0, -1) : basePath || ""}/client/register`;
    let body;
    body = JSON.stringify((0, smithy_client_7.take)(input, {
        clientName: [],
        clientType: [],
        scopes: (_) => (0, smithy_client_7._json)(_),
    }));
    return new protocol_http_2.HttpRequest({
        protocol,
        hostname,
        port,
        method: "POST",
        headers,
        path: resolvedPath,
        body,
    });
};
var se_StartDeviceAuthorizationCommand = async (input, context) => {
    const { hostname, protocol = "https", port, path: basePath } = await context.endpoint();
    const headers = {
        "content-type": "application/json",
    };
    const resolvedPath = `${(basePath === null || basePath === void 0 ? void 0 : basePath.endsWith("/")) ? basePath.slice(0, -1) : basePath || ""}/device_authorization`;
    let body;
    body = JSON.stringify((0, smithy_client_7.take)(input, {
        clientId: [],
        clientSecret: [],
        startUrl: [],
    }));
    return new protocol_http_2.HttpRequest({
        protocol,
        hostname,
        port,
        method: "POST",
        headers,
        path: resolvedPath,
        body,
    });
};
var de_CreateTokenCommand = async (output, context) => {
    if (output.statusCode !== 200 && output.statusCode >= 300) {
        return de_CreateTokenCommandError(output, context);
    }
    const contents = (0, smithy_client_7.map)({
        $metadata: deserializeMetadata(output),
    });
    const data = (0, smithy_client_7.expectNonNull)((0, smithy_client_7.expectObject)(await parseBody(output.body, context)), "body");
    const doc = (0, smithy_client_7.take)(data, {
        accessToken: smithy_client_7.expectString,
        expiresIn: smithy_client_7.expectInt32,
        idToken: smithy_client_7.expectString,
        refreshToken: smithy_client_7.expectString,
        tokenType: smithy_client_7.expectString,
    });
    Object.assign(contents, doc);
    return contents;
};
var de_CreateTokenCommandError = async (output, context) => {
    const parsedOutput = {
        ...output,
        body: await parseErrorBody(output.body, context),
    };
    const errorCode = loadRestJsonErrorCode(output, parsedOutput.body);
    switch (errorCode) {
        case "AccessDeniedException":
        case "com.amazonaws.ssooidc#AccessDeniedException":
            throw await de_AccessDeniedExceptionRes(parsedOutput, context);
        case "AuthorizationPendingException":
        case "com.amazonaws.ssooidc#AuthorizationPendingException":
            throw await de_AuthorizationPendingExceptionRes(parsedOutput, context);
        case "ExpiredTokenException":
        case "com.amazonaws.ssooidc#ExpiredTokenException":
            throw await de_ExpiredTokenExceptionRes(parsedOutput, context);
        case "InternalServerException":
        case "com.amazonaws.ssooidc#InternalServerException":
            throw await de_InternalServerExceptionRes(parsedOutput, context);
        case "InvalidClientException":
        case "com.amazonaws.ssooidc#InvalidClientException":
            throw await de_InvalidClientExceptionRes(parsedOutput, context);
        case "InvalidGrantException":
        case "com.amazonaws.ssooidc#InvalidGrantException":
            throw await de_InvalidGrantExceptionRes(parsedOutput, context);
        case "InvalidRequestException":
        case "com.amazonaws.ssooidc#InvalidRequestException":
            throw await de_InvalidRequestExceptionRes(parsedOutput, context);
        case "InvalidScopeException":
        case "com.amazonaws.ssooidc#InvalidScopeException":
            throw await de_InvalidScopeExceptionRes(parsedOutput, context);
        case "SlowDownException":
        case "com.amazonaws.ssooidc#SlowDownException":
            throw await de_SlowDownExceptionRes(parsedOutput, context);
        case "UnauthorizedClientException":
        case "com.amazonaws.ssooidc#UnauthorizedClientException":
            throw await de_UnauthorizedClientExceptionRes(parsedOutput, context);
        case "UnsupportedGrantTypeException":
        case "com.amazonaws.ssooidc#UnsupportedGrantTypeException":
            throw await de_UnsupportedGrantTypeExceptionRes(parsedOutput, context);
        default:
            const parsedBody = parsedOutput.body;
            return throwDefaultError({
                output,
                parsedBody,
                errorCode,
            });
    }
};
var de_RegisterClientCommand = async (output, context) => {
    if (output.statusCode !== 200 && output.statusCode >= 300) {
        return de_RegisterClientCommandError(output, context);
    }
    const contents = (0, smithy_client_7.map)({
        $metadata: deserializeMetadata(output),
    });
    const data = (0, smithy_client_7.expectNonNull)((0, smithy_client_7.expectObject)(await parseBody(output.body, context)), "body");
    const doc = (0, smithy_client_7.take)(data, {
        authorizationEndpoint: smithy_client_7.expectString,
        clientId: smithy_client_7.expectString,
        clientIdIssuedAt: smithy_client_7.expectLong,
        clientSecret: smithy_client_7.expectString,
        clientSecretExpiresAt: smithy_client_7.expectLong,
        tokenEndpoint: smithy_client_7.expectString,
    });
    Object.assign(contents, doc);
    return contents;
};
var de_RegisterClientCommandError = async (output, context) => {
    const parsedOutput = {
        ...output,
        body: await parseErrorBody(output.body, context),
    };
    const errorCode = loadRestJsonErrorCode(output, parsedOutput.body);
    switch (errorCode) {
        case "InternalServerException":
        case "com.amazonaws.ssooidc#InternalServerException":
            throw await de_InternalServerExceptionRes(parsedOutput, context);
        case "InvalidClientMetadataException":
        case "com.amazonaws.ssooidc#InvalidClientMetadataException":
            throw await de_InvalidClientMetadataExceptionRes(parsedOutput, context);
        case "InvalidRequestException":
        case "com.amazonaws.ssooidc#InvalidRequestException":
            throw await de_InvalidRequestExceptionRes(parsedOutput, context);
        case "InvalidScopeException":
        case "com.amazonaws.ssooidc#InvalidScopeException":
            throw await de_InvalidScopeExceptionRes(parsedOutput, context);
        default:
            const parsedBody = parsedOutput.body;
            return throwDefaultError({
                output,
                parsedBody,
                errorCode,
            });
    }
};
var de_StartDeviceAuthorizationCommand = async (output, context) => {
    if (output.statusCode !== 200 && output.statusCode >= 300) {
        return de_StartDeviceAuthorizationCommandError(output, context);
    }
    const contents = (0, smithy_client_7.map)({
        $metadata: deserializeMetadata(output),
    });
    const data = (0, smithy_client_7.expectNonNull)((0, smithy_client_7.expectObject)(await parseBody(output.body, context)), "body");
    const doc = (0, smithy_client_7.take)(data, {
        deviceCode: smithy_client_7.expectString,
        expiresIn: smithy_client_7.expectInt32,
        interval: smithy_client_7.expectInt32,
        userCode: smithy_client_7.expectString,
        verificationUri: smithy_client_7.expectString,
        verificationUriComplete: smithy_client_7.expectString,
    });
    Object.assign(contents, doc);
    return contents;
};
var de_StartDeviceAuthorizationCommandError = async (output, context) => {
    const parsedOutput = {
        ...output,
        body: await parseErrorBody(output.body, context),
    };
    const errorCode = loadRestJsonErrorCode(output, parsedOutput.body);
    switch (errorCode) {
        case "InternalServerException":
        case "com.amazonaws.ssooidc#InternalServerException":
            throw await de_InternalServerExceptionRes(parsedOutput, context);
        case "InvalidClientException":
        case "com.amazonaws.ssooidc#InvalidClientException":
            throw await de_InvalidClientExceptionRes(parsedOutput, context);
        case "InvalidRequestException":
        case "com.amazonaws.ssooidc#InvalidRequestException":
            throw await de_InvalidRequestExceptionRes(parsedOutput, context);
        case "SlowDownException":
        case "com.amazonaws.ssooidc#SlowDownException":
            throw await de_SlowDownExceptionRes(parsedOutput, context);
        case "UnauthorizedClientException":
        case "com.amazonaws.ssooidc#UnauthorizedClientException":
            throw await de_UnauthorizedClientExceptionRes(parsedOutput, context);
        default:
            const parsedBody = parsedOutput.body;
            return throwDefaultError({
                output,
                parsedBody,
                errorCode,
            });
    }
};
var throwDefaultError = (0, smithy_client_7.withBaseException)(SSOOIDCServiceException);
var de_AccessDeniedExceptionRes = async (parsedOutput, context) => {
    const contents = (0, smithy_client_7.map)({});
    const data = parsedOutput.body;
    const doc = (0, smithy_client_7.take)(data, {
        error: smithy_client_7.expectString,
        error_description: smithy_client_7.expectString,
    });
    Object.assign(contents, doc);
    const exception = new AccessDeniedException({
        $metadata: deserializeMetadata(parsedOutput),
        ...contents,
    });
    return (0, smithy_client_7.decorateServiceException)(exception, parsedOutput.body);
};
var de_AuthorizationPendingExceptionRes = async (parsedOutput, context) => {
    const contents = (0, smithy_client_7.map)({});
    const data = parsedOutput.body;
    const doc = (0, smithy_client_7.take)(data, {
        error: smithy_client_7.expectString,
        error_description: smithy_client_7.expectString,
    });
    Object.assign(contents, doc);
    const exception = new AuthorizationPendingException({
        $metadata: deserializeMetadata(parsedOutput),
        ...contents,
    });
    return (0, smithy_client_7.decorateServiceException)(exception, parsedOutput.body);
};
var de_ExpiredTokenExceptionRes = async (parsedOutput, context) => {
    const contents = (0, smithy_client_7.map)({});
    const data = parsedOutput.body;
    const doc = (0, smithy_client_7.take)(data, {
        error: smithy_client_7.expectString,
        error_description: smithy_client_7.expectString,
    });
    Object.assign(contents, doc);
    const exception = new ExpiredTokenException({
        $metadata: deserializeMetadata(parsedOutput),
        ...contents,
    });
    return (0, smithy_client_7.decorateServiceException)(exception, parsedOutput.body);
};
var de_InternalServerExceptionRes = async (parsedOutput, context) => {
    const contents = (0, smithy_client_7.map)({});
    const data = parsedOutput.body;
    const doc = (0, smithy_client_7.take)(data, {
        error: smithy_client_7.expectString,
        error_description: smithy_client_7.expectString,
    });
    Object.assign(contents, doc);
    const exception = new InternalServerException({
        $metadata: deserializeMetadata(parsedOutput),
        ...contents,
    });
    return (0, smithy_client_7.decorateServiceException)(exception, parsedOutput.body);
};
var de_InvalidClientExceptionRes = async (parsedOutput, context) => {
    const contents = (0, smithy_client_7.map)({});
    const data = parsedOutput.body;
    const doc = (0, smithy_client_7.take)(data, {
        error: smithy_client_7.expectString,
        error_description: smithy_client_7.expectString,
    });
    Object.assign(contents, doc);
    const exception = new InvalidClientException({
        $metadata: deserializeMetadata(parsedOutput),
        ...contents,
    });
    return (0, smithy_client_7.decorateServiceException)(exception, parsedOutput.body);
};
var de_InvalidClientMetadataExceptionRes = async (parsedOutput, context) => {
    const contents = (0, smithy_client_7.map)({});
    const data = parsedOutput.body;
    const doc = (0, smithy_client_7.take)(data, {
        error: smithy_client_7.expectString,
        error_description: smithy_client_7.expectString,
    });
    Object.assign(contents, doc);
    const exception = new InvalidClientMetadataException({
        $metadata: deserializeMetadata(parsedOutput),
        ...contents,
    });
    return (0, smithy_client_7.decorateServiceException)(exception, parsedOutput.body);
};
var de_InvalidGrantExceptionRes = async (parsedOutput, context) => {
    const contents = (0, smithy_client_7.map)({});
    const data = parsedOutput.body;
    const doc = (0, smithy_client_7.take)(data, {
        error: smithy_client_7.expectString,
        error_description: smithy_client_7.expectString,
    });
    Object.assign(contents, doc);
    const exception = new InvalidGrantException({
        $metadata: deserializeMetadata(parsedOutput),
        ...contents,
    });
    return (0, smithy_client_7.decorateServiceException)(exception, parsedOutput.body);
};
var de_InvalidRequestExceptionRes = async (parsedOutput, context) => {
    const contents = (0, smithy_client_7.map)({});
    const data = parsedOutput.body;
    const doc = (0, smithy_client_7.take)(data, {
        error: smithy_client_7.expectString,
        error_description: smithy_client_7.expectString,
    });
    Object.assign(contents, doc);
    const exception = new InvalidRequestException({
        $metadata: deserializeMetadata(parsedOutput),
        ...contents,
    });
    return (0, smithy_client_7.decorateServiceException)(exception, parsedOutput.body);
};
var de_InvalidScopeExceptionRes = async (parsedOutput, context) => {
    const contents = (0, smithy_client_7.map)({});
    const data = parsedOutput.body;
    const doc = (0, smithy_client_7.take)(data, {
        error: smithy_client_7.expectString,
        error_description: smithy_client_7.expectString,
    });
    Object.assign(contents, doc);
    const exception = new InvalidScopeException({
        $metadata: deserializeMetadata(parsedOutput),
        ...contents,
    });
    return (0, smithy_client_7.decorateServiceException)(exception, parsedOutput.body);
};
var de_SlowDownExceptionRes = async (parsedOutput, context) => {
    const contents = (0, smithy_client_7.map)({});
    const data = parsedOutput.body;
    const doc = (0, smithy_client_7.take)(data, {
        error: smithy_client_7.expectString,
        error_description: smithy_client_7.expectString,
    });
    Object.assign(contents, doc);
    const exception = new SlowDownException({
        $metadata: deserializeMetadata(parsedOutput),
        ...contents,
    });
    return (0, smithy_client_7.decorateServiceException)(exception, parsedOutput.body);
};
var de_UnauthorizedClientExceptionRes = async (parsedOutput, context) => {
    const contents = (0, smithy_client_7.map)({});
    const data = parsedOutput.body;
    const doc = (0, smithy_client_7.take)(data, {
        error: smithy_client_7.expectString,
        error_description: smithy_client_7.expectString,
    });
    Object.assign(contents, doc);
    const exception = new UnauthorizedClientException({
        $metadata: deserializeMetadata(parsedOutput),
        ...contents,
    });
    return (0, smithy_client_7.decorateServiceException)(exception, parsedOutput.body);
};
var de_UnsupportedGrantTypeExceptionRes = async (parsedOutput, context) => {
    const contents = (0, smithy_client_7.map)({});
    const data = parsedOutput.body;
    const doc = (0, smithy_client_7.take)(data, {
        error: smithy_client_7.expectString,
        error_description: smithy_client_7.expectString,
    });
    Object.assign(contents, doc);
    const exception = new UnsupportedGrantTypeException({
        $metadata: deserializeMetadata(parsedOutput),
        ...contents,
    });
    return (0, smithy_client_7.decorateServiceException)(exception, parsedOutput.body);
};
var deserializeMetadata = (output) => {
    var _a, _b;
    return ({
        httpStatusCode: output.statusCode,
        requestId: (_b = (_a = output.headers["x-amzn-requestid"]) !== null && _a !== void 0 ? _a : output.headers["x-amzn-request-id"]) !== null && _b !== void 0 ? _b : output.headers["x-amz-request-id"],
        extendedRequestId: output.headers["x-amz-id-2"],
        cfId: output.headers["x-amz-cf-id"],
    });
};
var collectBodyString = (streamBody, context) => (0, smithy_client_7.collectBody)(streamBody, context).then((body) => context.utf8Encoder(body));
var parseBody = (streamBody, context) => collectBodyString(streamBody, context).then((encoded) => {
    if (encoded.length) {
        return JSON.parse(encoded);
    }
    return {};
});
var parseErrorBody = async (errorBody, context) => {
    var _a;
    const value = await parseBody(errorBody, context);
    value.message = (_a = value.message) !== null && _a !== void 0 ? _a : value.Message;
    return value;
};
var loadRestJsonErrorCode = (output, data) => {
    const findKey = (object, key) => Object.keys(object).find((k2) => k2.toLowerCase() === key.toLowerCase());
    const sanitizeErrorCode = (rawValue) => {
        let cleanValue = rawValue;
        if (typeof cleanValue === "number") {
            cleanValue = cleanValue.toString();
        }
        if (cleanValue.indexOf(",") >= 0) {
            cleanValue = cleanValue.split(",")[0];
        }
        if (cleanValue.indexOf(":") >= 0) {
            cleanValue = cleanValue.split(":")[0];
        }
        if (cleanValue.indexOf("#") >= 0) {
            cleanValue = cleanValue.split("#")[1];
        }
        return cleanValue;
    };
    const headerKey = findKey(output.headers, "x-amzn-errortype");
    if (headerKey !== void 0) {
        return sanitizeErrorCode(output.headers[headerKey]);
    }
    if (data.code !== void 0) {
        return sanitizeErrorCode(data.code);
    }
    if (data["__type"] !== void 0) {
        return sanitizeErrorCode(data["__type"]);
    }
};
class CreateTokenCommand extends smithy_client_6.Command {
    constructor(input) {
        super();
        this.input = input;
    }
    static getEndpointParameterInstructions() {
        return {
            UseFIPS: { type: "builtInParams", name: "useFipsEndpoint" },
            Endpoint: { type: "builtInParams", name: "endpoint" },
            Region: { type: "builtInParams", name: "region" },
            UseDualStack: { type: "builtInParams", name: "useDualstackEndpoint" },
        };
    }
    resolveMiddleware(clientStack, configuration, options) {
        this.middlewareStack.use((0, middleware_serde_1.getSerdePlugin)(configuration, this.serialize, this.deserialize));
        this.middlewareStack.use((0, middleware_endpoint_2.getEndpointPlugin)(configuration, _CreateTokenCommand.getEndpointParameterInstructions()));
        const stack = clientStack.concat(this.middlewareStack);
        const { logger } = configuration;
        const clientName = "SSOOIDCClient";
        const commandName = "CreateTokenCommand";
        const handlerExecutionContext = {
            logger,
            clientName,
            commandName,
            inputFilterSensitiveLog: (_) => _,
            outputFilterSensitiveLog: (_) => _,
            [types_1.SMITHY_CONTEXT_KEY]: {
                service: "AWSSSOOIDCService",
                operation: "CreateToken",
            },
        };
        const { requestHandler } = configuration;
        return stack.resolve((request) => requestHandler.handle(request.request, options || {}), handlerExecutionContext);
    }
    serialize(input, context) {
        return se_CreateTokenCommand(input, context);
    }
    deserialize(output, context) {
        return de_CreateTokenCommand(output, context);
    }
}
exports.CreateTokenCommand = CreateTokenCommand;
const middleware_endpoint_3 = require("@smithy/middleware-endpoint");
const middleware_serde_2 = require("@smithy/middleware-serde");
const smithy_client_9 = require("@smithy/smithy-client");
const types_2 = require("@smithy/types");
class RegisterClientCommand extends smithy_client_9.Command {
    constructor(input) {
        super();
        this.input = input;
    }
    static getEndpointParameterInstructions() {
        return {
            UseFIPS: { type: "builtInParams", name: "useFipsEndpoint" },
            Endpoint: { type: "builtInParams", name: "endpoint" },
            Region: { type: "builtInParams", name: "region" },
            UseDualStack: { type: "builtInParams", name: "useDualstackEndpoint" },
        };
    }
    resolveMiddleware(clientStack, configuration, options) {
        this.middlewareStack.use((0, middleware_serde_2.getSerdePlugin)(configuration, this.serialize, this.deserialize));
        this.middlewareStack.use((0, middleware_endpoint_3.getEndpointPlugin)(configuration, _RegisterClientCommand.getEndpointParameterInstructions()));
        const stack = clientStack.concat(this.middlewareStack);
        const { logger } = configuration;
        const clientName = "SSOOIDCClient";
        const commandName = "RegisterClientCommand";
        const handlerExecutionContext = {
            logger,
            clientName,
            commandName,
            inputFilterSensitiveLog: (_) => _,
            outputFilterSensitiveLog: (_) => _,
            [types_2.SMITHY_CONTEXT_KEY]: {
                service: "AWSSSOOIDCService",
                operation: "RegisterClient",
            },
        };
        const { requestHandler } = configuration;
        return stack.resolve((request) => requestHandler.handle(request.request, options || {}), handlerExecutionContext);
    }
    serialize(input, context) {
        return se_RegisterClientCommand(input, context);
    }
    deserialize(output, context) {
        return de_RegisterClientCommand(output, context);
    }
}
const middleware_endpoint_4 = require("@smithy/middleware-endpoint");
const middleware_serde_3 = require("@smithy/middleware-serde");
const smithy_client_10 = require("@smithy/smithy-client");
const types_3 = require("@smithy/types");
class StartDeviceAuthorizationCommand extends smithy_client_10.Command {
    constructor(input) {
        super();
        this.input = input;
    }
    static getEndpointParameterInstructions() {
        return {
            UseFIPS: { type: "builtInParams", name: "useFipsEndpoint" },
            Endpoint: { type: "builtInParams", name: "endpoint" },
            Region: { type: "builtInParams", name: "region" },
            UseDualStack: { type: "builtInParams", name: "useDualstackEndpoint" },
        };
    }
    resolveMiddleware(clientStack, configuration, options) {
        this.middlewareStack.use((0, middleware_serde_3.getSerdePlugin)(configuration, this.serialize, this.deserialize));
        this.middlewareStack.use((0, middleware_endpoint_4.getEndpointPlugin)(configuration, _StartDeviceAuthorizationCommand.getEndpointParameterInstructions()));
        const stack = clientStack.concat(this.middlewareStack);
        const { logger } = configuration;
        const clientName = "SSOOIDCClient";
        const commandName = "StartDeviceAuthorizationCommand";
        const handlerExecutionContext = {
            logger,
            clientName,
            commandName,
            inputFilterSensitiveLog: (_) => _,
            outputFilterSensitiveLog: (_) => _,
            [types_3.SMITHY_CONTEXT_KEY]: {
                service: "AWSSSOOIDCService",
                operation: "StartDeviceAuthorization",
            },
        };
        const { requestHandler } = configuration;
        return stack.resolve((request) => requestHandler.handle(request.request, options || {}), handlerExecutionContext);
    }
    serialize(input, context) {
        return se_StartDeviceAuthorizationCommand(input, context);
    }
    deserialize(output, context) {
        return de_StartDeviceAuthorizationCommand(output, context);
    }
}
var commands = {
    CreateTokenCommand,
    RegisterClientCommand,
    StartDeviceAuthorizationCommand,
};
var SSOOIDC = class extends SSOOIDCClient {
};
(0, smithy_client_5.createAggregatedClient)(commands, SSOOIDC);
