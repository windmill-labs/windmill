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
  fromIni: () => fromIni
});
module.exports = __toCommonJS(src_exports);

// src/fromIni.ts


// src/resolveProfileData.ts


// src/resolveAssumeRoleCredentials.ts

var import_shared_ini_file_loader = require("@smithy/shared-ini-file-loader");

// src/resolveCredentialSource.ts
var import_credential_provider_env = require("@aws-sdk/credential-provider-env");
var import_credential_provider_imds = require("@smithy/credential-provider-imds");
var import_property_provider = require("@smithy/property-provider");
var resolveCredentialSource = /* @__PURE__ */ __name((credentialSource, profileName) => {
  const sourceProvidersMap = {
    EcsContainer: import_credential_provider_imds.fromContainerMetadata,
    Ec2InstanceMetadata: import_credential_provider_imds.fromInstanceMetadata,
    Environment: import_credential_provider_env.fromEnv
  };
  if (credentialSource in sourceProvidersMap) {
    return sourceProvidersMap[credentialSource]();
  } else {
    throw new import_property_provider.CredentialsProviderError(
      `Unsupported credential source in profile ${profileName}. Got ${credentialSource}, expected EcsContainer or Ec2InstanceMetadata or Environment.`
    );
  }
}, "resolveCredentialSource");

// src/resolveAssumeRoleCredentials.ts
var isAssumeRoleProfile = /* @__PURE__ */ __name((arg) => Boolean(arg) && typeof arg === "object" && typeof arg.role_arn === "string" && ["undefined", "string"].indexOf(typeof arg.role_session_name) > -1 && ["undefined", "string"].indexOf(typeof arg.external_id) > -1 && ["undefined", "string"].indexOf(typeof arg.mfa_serial) > -1 && (isAssumeRoleWithSourceProfile(arg) || isAssumeRoleWithProviderProfile(arg)), "isAssumeRoleProfile");
var isAssumeRoleWithSourceProfile = /* @__PURE__ */ __name((arg) => typeof arg.source_profile === "string" && typeof arg.credential_source === "undefined", "isAssumeRoleWithSourceProfile");
var isAssumeRoleWithProviderProfile = /* @__PURE__ */ __name((arg) => typeof arg.credential_source === "string" && typeof arg.source_profile === "undefined", "isAssumeRoleWithProviderProfile");
var resolveAssumeRoleCredentials = /* @__PURE__ */ __name(async (profileName, profiles, options, visitedProfiles = {}) => {
  const data = profiles[profileName];
  if (!options.roleAssumer) {
    throw new import_property_provider.CredentialsProviderError(
      `Profile ${profileName} requires a role to be assumed, but no role assumption callback was provided.`,
      false
    );
  }
  const { source_profile } = data;
  if (source_profile && source_profile in visitedProfiles) {
    throw new import_property_provider.CredentialsProviderError(
      `Detected a cycle attempting to resolve credentials for profile ${(0, import_shared_ini_file_loader.getProfileName)(options)}. Profiles visited: ` + Object.keys(visitedProfiles).join(", "),
      false
    );
  }
  const sourceCredsProvider = source_profile ? resolveProfileData(source_profile, profiles, options, {
    ...visitedProfiles,
    [source_profile]: true
  }) : resolveCredentialSource(data.credential_source, profileName)();
  const params = {
    RoleArn: data.role_arn,
    RoleSessionName: data.role_session_name || `aws-sdk-js-${Date.now()}`,
    ExternalId: data.external_id,
    DurationSeconds: parseInt(data.duration_seconds || "3600", 10)
  };
  const { mfa_serial } = data;
  if (mfa_serial) {
    if (!options.mfaCodeProvider) {
      throw new import_property_provider.CredentialsProviderError(
        `Profile ${profileName} requires multi-factor authentication, but no MFA code callback was provided.`,
        false
      );
    }
    params.SerialNumber = mfa_serial;
    params.TokenCode = await options.mfaCodeProvider(mfa_serial);
  }
  const sourceCreds = await sourceCredsProvider;
  return options.roleAssumer(sourceCreds, params);
}, "resolveAssumeRoleCredentials");

// src/resolveProcessCredentials.ts
var import_credential_provider_process = require("@aws-sdk/credential-provider-process");
var isProcessProfile = /* @__PURE__ */ __name((arg) => Boolean(arg) && typeof arg === "object" && typeof arg.credential_process === "string", "isProcessProfile");
var resolveProcessCredentials = /* @__PURE__ */ __name(async (options, profile) => (0, import_credential_provider_process.fromProcess)({
  ...options,
  profile
})(), "resolveProcessCredentials");

// src/resolveSsoCredentials.ts
var import_credential_provider_sso = require("@aws-sdk/credential-provider-sso");

var resolveSsoCredentials = /* @__PURE__ */ __name((data) => {
  const { sso_start_url, sso_account_id, sso_session, sso_region, sso_role_name } = (0, import_credential_provider_sso.validateSsoProfile)(data);
  return (0, import_credential_provider_sso.fromSSO)({
    ssoStartUrl: sso_start_url,
    ssoAccountId: sso_account_id,
    ssoSession: sso_session,
    ssoRegion: sso_region,
    ssoRoleName: sso_role_name
  })();
}, "resolveSsoCredentials");

// src/resolveStaticCredentials.ts
var isStaticCredsProfile = /* @__PURE__ */ __name((arg) => Boolean(arg) && typeof arg === "object" && typeof arg.aws_access_key_id === "string" && typeof arg.aws_secret_access_key === "string" && ["undefined", "string"].indexOf(typeof arg.aws_session_token) > -1, "isStaticCredsProfile");
var resolveStaticCredentials = /* @__PURE__ */ __name((profile) => Promise.resolve({
  accessKeyId: profile.aws_access_key_id,
  secretAccessKey: profile.aws_secret_access_key,
  sessionToken: profile.aws_session_token,
  credentialScope: profile.aws_credential_scope
}), "resolveStaticCredentials");

// src/resolveWebIdentityCredentials.ts
var import_credential_provider_web_identity = require("@aws-sdk/credential-provider-web-identity");
var isWebIdentityProfile = /* @__PURE__ */ __name((arg) => Boolean(arg) && typeof arg === "object" && typeof arg.web_identity_token_file === "string" && typeof arg.role_arn === "string" && ["undefined", "string"].indexOf(typeof arg.role_session_name) > -1, "isWebIdentityProfile");
var resolveWebIdentityCredentials = /* @__PURE__ */ __name(async (profile, options) => (0, import_credential_provider_web_identity.fromTokenFile)({
  webIdentityTokenFile: profile.web_identity_token_file,
  roleArn: profile.role_arn,
  roleSessionName: profile.role_session_name,
  roleAssumerWithWebIdentity: options.roleAssumerWithWebIdentity
})(), "resolveWebIdentityCredentials");

// src/resolveProfileData.ts
var resolveProfileData = /* @__PURE__ */ __name(async (profileName, profiles, options, visitedProfiles = {}) => {
  const data = profiles[profileName];
  if (Object.keys(visitedProfiles).length > 0 && isStaticCredsProfile(data)) {
    return resolveStaticCredentials(data);
  }
  if (isAssumeRoleProfile(data)) {
    return resolveAssumeRoleCredentials(profileName, profiles, options, visitedProfiles);
  }
  if (isStaticCredsProfile(data)) {
    return resolveStaticCredentials(data);
  }
  if (isWebIdentityProfile(data)) {
    return resolveWebIdentityCredentials(data, options);
  }
  if (isProcessProfile(data)) {
    return resolveProcessCredentials(options, profileName);
  }
  if ((0, import_credential_provider_sso.isSsoProfile)(data)) {
    return resolveSsoCredentials(data);
  }
  throw new import_property_provider.CredentialsProviderError(`Profile ${profileName} could not be found or parsed in shared credentials file.`);
}, "resolveProfileData");

// src/fromIni.ts
var fromIni = /* @__PURE__ */ __name((init = {}) => async () => {
  const profiles = await (0, import_shared_ini_file_loader.parseKnownFiles)(init);
  return resolveProfileData((0, import_shared_ini_file_loader.getProfileName)(init), profiles, init);
}, "fromIni");
// Annotate the CommonJS export names for ESM import in node:

0 && (module.exports = {
  fromIni
});

