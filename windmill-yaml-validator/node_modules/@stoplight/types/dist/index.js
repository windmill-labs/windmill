'use strict';

Object.defineProperty(exports, '__esModule', { value: true });

exports.HttpOperationSecurityDeclarationTypes = void 0;
(function (HttpOperationSecurityDeclarationTypes) {
    /** Indicates that the operation has no security declarations. */
    HttpOperationSecurityDeclarationTypes["None"] = "none";
    /** Indicates that the operation has explicit security declarations. */
    HttpOperationSecurityDeclarationTypes["Declared"] = "declared";
    /** Indicates that the operation inherits its security declarations from the service. */
    HttpOperationSecurityDeclarationTypes["InheritedFromService"] = "inheritedFromService";
})(exports.HttpOperationSecurityDeclarationTypes || (exports.HttpOperationSecurityDeclarationTypes = {}));
exports.HttpParamStyles = void 0;
(function (HttpParamStyles) {
    /** Used when OAS2 type !== array */
    HttpParamStyles["Unspecified"] = "unspecified";
    /**
     * OAS 3.x style simple
     * OAS 2 collectionFormat csv
     */
    HttpParamStyles["Simple"] = "simple";
    /**
     * OAS 3.x style matrix
     * OAS 2 collectionFormat no support
     */
    HttpParamStyles["Matrix"] = "matrix";
    /**
     * OAS 3.x style label
     * OAS 2 collectionFormat no support
     */
    HttpParamStyles["Label"] = "label";
    /**
     * OAS 3.x style form
     * OAS 2 collectionFormat
     *   * csv, when explode === false
     *   * multi, when explode === true
     */
    HttpParamStyles["Form"] = "form";
    /**
     * OAS 3.x no support
     * OAS 2 collectionFormat csv when explode === undefined
     */
    HttpParamStyles["CommaDelimited"] = "commaDelimited";
    /**
     * OAS 3.x style spaceDelimited
     * OAS 2 collectionFormat ssv
     */
    HttpParamStyles["SpaceDelimited"] = "spaceDelimited";
    /**
     * OAS 3.x style spaceDelimited
     * OAS 2 collectionFormat pipes
     */
    HttpParamStyles["PipeDelimited"] = "pipeDelimited";
    /**
     * OAS 3.x style deepObject
     * OAS 2 collectionFormat no support
     */
    HttpParamStyles["DeepObject"] = "deepObject";
    /**
     * OAS 3.x style no support
     * OAS 2 collectionFormat tsv
     */
    HttpParamStyles["TabDelimited"] = "tabDelimited";
})(exports.HttpParamStyles || (exports.HttpParamStyles = {}));

/**
 * Represents the severity of diagnostics.
 */
exports.DiagnosticSeverity = void 0;
(function (DiagnosticSeverity) {
    /**
     * Something not allowed by the rules of a language or other means.
     */
    DiagnosticSeverity[DiagnosticSeverity["Error"] = 0] = "Error";
    /**
     * Something suspicious but allowed.
     */
    DiagnosticSeverity[DiagnosticSeverity["Warning"] = 1] = "Warning";
    /**
     * Something to inform about but not a problem.
     */
    DiagnosticSeverity[DiagnosticSeverity["Information"] = 2] = "Information";
    /**
     * Something to hint to a better way of doing it, like proposing
     * a refactoring.
     */
    DiagnosticSeverity[DiagnosticSeverity["Hint"] = 3] = "Hint";
})(exports.DiagnosticSeverity || (exports.DiagnosticSeverity = {}));

/**
 * Stoplight node types
 */
exports.NodeType = void 0;
(function (NodeType) {
    NodeType["Article"] = "article";
    NodeType["HttpService"] = "http_service";
    NodeType["HttpServer"] = "http_server";
    NodeType["HttpOperation"] = "http_operation";
    NodeType["HttpCallback"] = "http_callback";
    NodeType["HttpWebhook"] = "http_webhook";
    NodeType["Model"] = "model";
    NodeType["Generic"] = "generic";
    NodeType["Unknown"] = "unknown";
    NodeType["TableOfContents"] = "table_of_contents";
    NodeType["SpectralRuleset"] = "spectral_ruleset";
    NodeType["Styleguide"] = "styleguide";
    NodeType["Image"] = "image";
    NodeType["StoplightResolutions"] = "stoplight_resolutions";
    NodeType["StoplightOverride"] = "stoplight_override";
})(exports.NodeType || (exports.NodeType = {}));
/**
 * Node data formats
 */
exports.NodeFormat = void 0;
(function (NodeFormat) {
    NodeFormat["Json"] = "json";
    NodeFormat["Markdown"] = "markdown";
    NodeFormat["Yaml"] = "yaml";
    NodeFormat["Javascript"] = "javascript";
    NodeFormat["Apng"] = "apng";
    NodeFormat["Avif"] = "avif";
    NodeFormat["Bmp"] = "bmp";
    NodeFormat["Gif"] = "gif";
    NodeFormat["Jpeg"] = "jpeg";
    NodeFormat["Png"] = "png";
    NodeFormat["Svg"] = "svg";
    NodeFormat["Webp"] = "webp";
})(exports.NodeFormat || (exports.NodeFormat = {}));
