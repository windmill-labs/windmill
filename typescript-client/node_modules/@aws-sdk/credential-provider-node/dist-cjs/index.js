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
  defaultProvider: () => defaultProvider
});
module.exports = __toCommonJS(src_exports);

// src/defaultProvider.ts
var import_credential_provider_env = require("@aws-sdk/credential-provider-env");
var import_credential_provider_ini = require("@aws-sdk/credential-provider-ini");
var import_credential_provider_process = require("@aws-sdk/credential-provider-process");
var import_credential_provider_sso = require("@aws-sdk/credential-provider-sso");
var import_credential_provider_web_identity = require("@aws-sdk/credential-provider-web-identity");

var import_shared_ini_file_loader = require("@smithy/shared-ini-file-loader");

// src/remoteProvider.ts
var import_credential_provider_imds = require("@smithy/credential-provider-imds");
var import_property_provider = require("@smithy/property-provider");
var ENV_IMDS_DISABLED = "AWS_EC2_METADATA_DISABLED";
var remoteProvider = /* @__PURE__ */ __name((init) => {
  if (process.env[import_credential_provider_imds.ENV_CMDS_RELATIVE_URI] || process.env[import_credential_provider_imds.ENV_CMDS_FULL_URI]) {
    return (0, import_credential_provider_imds.fromContainerMetadata)(init);
  }
  if (process.env[ENV_IMDS_DISABLED]) {
    return async () => {
      throw new import_property_provider.CredentialsProviderError("EC2 Instance Metadata Service access disabled");
    };
  }
  return (0, import_credential_provider_imds.fromInstanceMetadata)(init);
}, "remoteProvider");

// src/defaultProvider.ts
var defaultProvider = /* @__PURE__ */ __name((init = {}) => (0, import_property_provider.memoize)(
  (0, import_property_provider.chain)(
    ...init.profile || process.env[import_shared_ini_file_loader.ENV_PROFILE] ? [] : [(0, import_credential_provider_env.fromEnv)()],
    (0, import_credential_provider_sso.fromSSO)(init),
    (0, import_credential_provider_ini.fromIni)(init),
    (0, import_credential_provider_process.fromProcess)(init),
    (0, import_credential_provider_web_identity.fromTokenFile)(init),
    remoteProvider(init),
    async () => {
      throw new import_property_provider.CredentialsProviderError("Could not load credentials from any providers", false);
    }
  ),
  (credentials) => credentials.expiration !== void 0 && credentials.expiration.getTime() - Date.now() < 3e5,
  (credentials) => credentials.expiration !== void 0
), "defaultProvider");
// Annotate the CommonJS export names for ESM import in node:

0 && (module.exports = {
  defaultProvider
});

