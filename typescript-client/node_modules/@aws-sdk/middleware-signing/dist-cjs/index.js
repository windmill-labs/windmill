var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __name = (target, value) => __defProp(target, "name", { value, configurable: true });
var __export = (target, all) => {
  for (var name in all)
    __defProp(target, name, { get: all[name], enumerable: true });
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toCommonJS = (mod) => __copyProps(__defProp({}, "__esModule", { value: true }), mod);

// src/index.ts
var src_exports = {};
__export(src_exports, {
  awsAuthMiddleware: () => awsAuthMiddleware,
  awsAuthMiddlewareOptions: () => awsAuthMiddlewareOptions,
  getAwsAuthPlugin: () => getAwsAuthPlugin,
  getSigV4AuthPlugin: () => getSigV4AuthPlugin,
  resolveAwsAuthConfig: () => resolveAwsAuthConfig,
  resolveSigV4AuthConfig: () => resolveSigV4AuthConfig
});
module.exports = __toCommonJS(src_exports);

// src/awsAuthConfiguration.ts
var import_property_provider = require("@smithy/property-provider");
var import_signature_v4 = require("@smithy/signature-v4");
var import_util_middleware = require("@smithy/util-middleware");
var CREDENTIAL_EXPIRE_WINDOW = 3e5;
var resolveAwsAuthConfig = /* @__PURE__ */ __name((input) => {
  const normalizedCreds = input.credentials ? normalizeCredentialProvider(input.credentials) : input.credentialDefaultProvider(input);
  const { signingEscapePath = true, systemClockOffset = input.systemClockOffset || 0, sha256 } = input;
  let signer;
  if (input.signer) {
    signer = (0, import_util_middleware.normalizeProvider)(input.signer);
  } else if (input.regionInfoProvider) {
    signer = /* @__PURE__ */ __name(() => (0, import_util_middleware.normalizeProvider)(input.region)().then(
      async (region) => [
        await input.regionInfoProvider(region, {
          useFipsEndpoint: await input.useFipsEndpoint(),
          useDualstackEndpoint: await input.useDualstackEndpoint()
        }) || {},
        region
      ]
    ).then(([regionInfo, region]) => {
      const { signingRegion, signingService } = regionInfo;
      input.signingRegion = input.signingRegion || signingRegion || region;
      input.signingName = input.signingName || signingService || input.serviceId;
      const params = {
        ...input,
        credentials: normalizedCreds,
        region: input.signingRegion,
        service: input.signingName,
        sha256,
        uriEscapePath: signingEscapePath
      };
      const SignerCtor = input.signerConstructor || import_signature_v4.SignatureV4;
      return new SignerCtor(params);
    }), "signer");
  } else {
    signer = /* @__PURE__ */ __name(async (authScheme) => {
      authScheme = Object.assign(
        {},
        {
          name: "sigv4",
          signingName: input.signingName || input.defaultSigningName,
          signingRegion: await (0, import_util_middleware.normalizeProvider)(input.region)(),
          properties: {}
        },
        authScheme
      );
      const signingRegion = authScheme.signingRegion;
      const signingService = authScheme.signingName;
      input.signingRegion = input.signingRegion || signingRegion;
      input.signingName = input.signingName || signingService || input.serviceId;
      const params = {
        ...input,
        credentials: normalizedCreds,
        region: input.signingRegion,
        service: input.signingName,
        sha256,
        uriEscapePath: signingEscapePath
      };
      const SignerCtor = input.signerConstructor || import_signature_v4.SignatureV4;
      return new SignerCtor(params);
    }, "signer");
  }
  return {
    ...input,
    systemClockOffset,
    signingEscapePath,
    credentials: normalizedCreds,
    signer
  };
}, "resolveAwsAuthConfig");
var resolveSigV4AuthConfig = /* @__PURE__ */ __name((input) => {
  const normalizedCreds = input.credentials ? normalizeCredentialProvider(input.credentials) : input.credentialDefaultProvider(input);
  const { signingEscapePath = true, systemClockOffset = input.systemClockOffset || 0, sha256 } = input;
  let signer;
  if (input.signer) {
    signer = (0, import_util_middleware.normalizeProvider)(input.signer);
  } else {
    signer = (0, import_util_middleware.normalizeProvider)(
      new import_signature_v4.SignatureV4({
        credentials: normalizedCreds,
        region: input.region,
        service: input.signingName,
        sha256,
        uriEscapePath: signingEscapePath
      })
    );
  }
  return {
    ...input,
    systemClockOffset,
    signingEscapePath,
    credentials: normalizedCreds,
    signer
  };
}, "resolveSigV4AuthConfig");
var normalizeCredentialProvider = /* @__PURE__ */ __name((credentials) => {
  if (typeof credentials === "function") {
    return (0, import_property_provider.memoize)(
      credentials,
      (credentials2) => credentials2.expiration !== void 0 && credentials2.expiration.getTime() - Date.now() < CREDENTIAL_EXPIRE_WINDOW,
      (credentials2) => credentials2.expiration !== void 0
    );
  }
  return (0, import_util_middleware.normalizeProvider)(credentials);
}, "normalizeCredentialProvider");

// src/awsAuthMiddleware.ts
var import_protocol_http = require("@smithy/protocol-http");

// src/utils/getSkewCorrectedDate.ts
var getSkewCorrectedDate = /* @__PURE__ */ __name((systemClockOffset) => new Date(Date.now() + systemClockOffset), "getSkewCorrectedDate");

// src/utils/isClockSkewed.ts
var isClockSkewed = /* @__PURE__ */ __name((clockTime, systemClockOffset) => Math.abs(getSkewCorrectedDate(systemClockOffset).getTime() - clockTime) >= 3e5, "isClockSkewed");

// src/utils/getUpdatedSystemClockOffset.ts
var getUpdatedSystemClockOffset = /* @__PURE__ */ __name((clockTime, currentSystemClockOffset) => {
  const clockTimeInMs = Date.parse(clockTime);
  if (isClockSkewed(clockTimeInMs, currentSystemClockOffset)) {
    return clockTimeInMs - Date.now();
  }
  return currentSystemClockOffset;
}, "getUpdatedSystemClockOffset");

// src/awsAuthMiddleware.ts
var awsAuthMiddleware = /* @__PURE__ */ __name((options) => (next, context) => async function(args) {
  var _a, _b, _c, _d;
  if (!import_protocol_http.HttpRequest.isInstance(args.request))
    return next(args);
  const authScheme = (_c = (_b = (_a = context.endpointV2) == null ? void 0 : _a.properties) == null ? void 0 : _b.authSchemes) == null ? void 0 : _c[0];
  const multiRegionOverride = (authScheme == null ? void 0 : authScheme.name) === "sigv4a" ? (_d = authScheme == null ? void 0 : authScheme.signingRegionSet) == null ? void 0 : _d.join(",") : void 0;
  const signer = await options.signer(authScheme);
  let signedRequest;
  const signingOptions = {
    signingDate: getSkewCorrectedDate(options.systemClockOffset),
    signingRegion: multiRegionOverride || context["signing_region"],
    signingService: context["signing_service"]
  };
  if (context.s3ExpressIdentity) {
    const sigV4MultiRegion = signer;
    signedRequest = await sigV4MultiRegion.signWithCredentials(
      args.request,
      context.s3ExpressIdentity,
      signingOptions
    );
    if (signedRequest.headers["X-Amz-Security-Token"] || signedRequest.headers["x-amz-security-token"]) {
      throw new Error("X-Amz-Security-Token must not be set for s3-express requests.");
    }
  } else {
    signedRequest = await signer.sign(args.request, signingOptions);
  }
  const output = await next({
    ...args,
    request: signedRequest
  }).catch((error) => {
    const serverTime = error.ServerTime ?? getDateHeader(error.$response);
    if (serverTime) {
      options.systemClockOffset = getUpdatedSystemClockOffset(serverTime, options.systemClockOffset);
    }
    throw error;
  });
  const dateHeader = getDateHeader(output.response);
  if (dateHeader) {
    options.systemClockOffset = getUpdatedSystemClockOffset(dateHeader, options.systemClockOffset);
  }
  return output;
}, "awsAuthMiddleware");
var getDateHeader = /* @__PURE__ */ __name((response) => {
  var _a, _b;
  return import_protocol_http.HttpResponse.isInstance(response) ? ((_a = response.headers) == null ? void 0 : _a.date) ?? ((_b = response.headers) == null ? void 0 : _b.Date) : void 0;
}, "getDateHeader");
var awsAuthMiddlewareOptions = {
  name: "awsAuthMiddleware",
  tags: ["SIGNATURE", "AWSAUTH"],
  relation: "after",
  toMiddleware: "retryMiddleware",
  override: true
};
var getAwsAuthPlugin = /* @__PURE__ */ __name((options) => ({
  applyToStack: (clientStack) => {
    clientStack.addRelativeTo(awsAuthMiddleware(options), awsAuthMiddlewareOptions);
  }
}), "getAwsAuthPlugin");
var getSigV4AuthPlugin = getAwsAuthPlugin;
// Annotate the CommonJS export names for ESM import in node:

0 && (module.exports = {
  awsAuthMiddleware,
  awsAuthMiddlewareOptions,
  getAwsAuthPlugin,
  getSigV4AuthPlugin,
  resolveAwsAuthConfig,
  resolveSigV4AuthConfig
});

