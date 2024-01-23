"use strict";
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
var __reExport = (target, mod, secondTarget) => (__copyProps(target, mod, "default"), secondTarget && __copyProps(secondTarget, mod, "default"));
var __toCommonJS = (mod) => __copyProps(__defProp({}, "__esModule", { value: true }), mod);

// src/index.ts
var src_exports = {};
__export(src_exports, {
  AssumeRoleWithSAMLCommand: () => AssumeRoleWithSAMLCommand,
  ClientInputEndpointParameters: () => import_EndpointParameters7.ClientInputEndpointParameters,
  DecodeAuthorizationMessageCommand: () => DecodeAuthorizationMessageCommand,
  GetAccessKeyInfoCommand: () => GetAccessKeyInfoCommand,
  GetCallerIdentityCommand: () => GetCallerIdentityCommand,
  GetFederationTokenCommand: () => GetFederationTokenCommand,
  GetSessionTokenCommand: () => GetSessionTokenCommand,
  RuntimeExtension: () => import_runtimeExtensions.RuntimeExtension,
  STS: () => STS,
  STSServiceException: () => import_STSServiceException.STSServiceException,
  decorateDefaultCredentialProvider: () => decorateDefaultCredentialProvider,
  getDefaultRoleAssumer: () => getDefaultRoleAssumer,
  getDefaultRoleAssumerWithWebIdentity: () => getDefaultRoleAssumerWithWebIdentity
});
module.exports = __toCommonJS(src_exports);
__reExport(src_exports, require("././STSClient"), module.exports);

// src/STS.ts

var import_AssumeRoleCommand = require("./commands/AssumeRoleCommand");

// src/commands/AssumeRoleWithSAMLCommand.ts
var import_middleware_endpoint = require("@smithy/middleware-endpoint");
var import_middleware_serde = require("@smithy/middleware-serde");
var import_smithy_client = require("@smithy/smithy-client");
var import_types = require("@smithy/types");
var import_EndpointParameters = require("./endpoint/EndpointParameters");
var import_models_0 = require("./models/models_0");
var import_Aws_query = require("./protocols/Aws_query");
var _AssumeRoleWithSAMLCommand = class _AssumeRoleWithSAMLCommand extends import_smithy_client.Command.classBuilder().ep({
  ...import_EndpointParameters.commonParams
}).m(function(Command, cs, config, o) {
  return [
    (0, import_middleware_serde.getSerdePlugin)(config, this.serialize, this.deserialize),
    (0, import_middleware_endpoint.getEndpointPlugin)(config, Command.getEndpointParameterInstructions())
  ];
}).s("AWSSecurityTokenServiceV20110615", "AssumeRoleWithSAML", {}).n("STSClient", "AssumeRoleWithSAMLCommand").f(import_models_0.AssumeRoleWithSAMLRequestFilterSensitiveLog, import_models_0.AssumeRoleWithSAMLResponseFilterSensitiveLog).ser(import_Aws_query.se_AssumeRoleWithSAMLCommand).de(import_Aws_query.de_AssumeRoleWithSAMLCommand).build() {
};
__name(_AssumeRoleWithSAMLCommand, "AssumeRoleWithSAMLCommand");
var AssumeRoleWithSAMLCommand = _AssumeRoleWithSAMLCommand;

// src/STS.ts
var import_AssumeRoleWithWebIdentityCommand = require("./commands/AssumeRoleWithWebIdentityCommand");

// src/commands/DecodeAuthorizationMessageCommand.ts




var import_EndpointParameters2 = require("./endpoint/EndpointParameters");
var import_Aws_query2 = require("./protocols/Aws_query");
var _DecodeAuthorizationMessageCommand = class _DecodeAuthorizationMessageCommand extends import_smithy_client.Command.classBuilder().ep({
  ...import_EndpointParameters2.commonParams
}).m(function(Command, cs, config, o) {
  return [
    (0, import_middleware_serde.getSerdePlugin)(config, this.serialize, this.deserialize),
    (0, import_middleware_endpoint.getEndpointPlugin)(config, Command.getEndpointParameterInstructions())
  ];
}).s("AWSSecurityTokenServiceV20110615", "DecodeAuthorizationMessage", {}).n("STSClient", "DecodeAuthorizationMessageCommand").f(void 0, void 0).ser(import_Aws_query2.se_DecodeAuthorizationMessageCommand).de(import_Aws_query2.de_DecodeAuthorizationMessageCommand).build() {
};
__name(_DecodeAuthorizationMessageCommand, "DecodeAuthorizationMessageCommand");
var DecodeAuthorizationMessageCommand = _DecodeAuthorizationMessageCommand;

// src/commands/GetAccessKeyInfoCommand.ts




var import_EndpointParameters3 = require("./endpoint/EndpointParameters");
var import_Aws_query3 = require("./protocols/Aws_query");
var _GetAccessKeyInfoCommand = class _GetAccessKeyInfoCommand extends import_smithy_client.Command.classBuilder().ep({
  ...import_EndpointParameters3.commonParams
}).m(function(Command, cs, config, o) {
  return [
    (0, import_middleware_serde.getSerdePlugin)(config, this.serialize, this.deserialize),
    (0, import_middleware_endpoint.getEndpointPlugin)(config, Command.getEndpointParameterInstructions())
  ];
}).s("AWSSecurityTokenServiceV20110615", "GetAccessKeyInfo", {}).n("STSClient", "GetAccessKeyInfoCommand").f(void 0, void 0).ser(import_Aws_query3.se_GetAccessKeyInfoCommand).de(import_Aws_query3.de_GetAccessKeyInfoCommand).build() {
};
__name(_GetAccessKeyInfoCommand, "GetAccessKeyInfoCommand");
var GetAccessKeyInfoCommand = _GetAccessKeyInfoCommand;

// src/commands/GetCallerIdentityCommand.ts




var import_EndpointParameters4 = require("./endpoint/EndpointParameters");
var import_Aws_query4 = require("./protocols/Aws_query");
var _GetCallerIdentityCommand = class _GetCallerIdentityCommand extends import_smithy_client.Command.classBuilder().ep({
  ...import_EndpointParameters4.commonParams
}).m(function(Command, cs, config, o) {
  return [
    (0, import_middleware_serde.getSerdePlugin)(config, this.serialize, this.deserialize),
    (0, import_middleware_endpoint.getEndpointPlugin)(config, Command.getEndpointParameterInstructions())
  ];
}).s("AWSSecurityTokenServiceV20110615", "GetCallerIdentity", {}).n("STSClient", "GetCallerIdentityCommand").f(void 0, void 0).ser(import_Aws_query4.se_GetCallerIdentityCommand).de(import_Aws_query4.de_GetCallerIdentityCommand).build() {
};
__name(_GetCallerIdentityCommand, "GetCallerIdentityCommand");
var GetCallerIdentityCommand = _GetCallerIdentityCommand;

// src/commands/GetFederationTokenCommand.ts




var import_EndpointParameters5 = require("./endpoint/EndpointParameters");
var import_models_02 = require("./models/models_0");
var import_Aws_query5 = require("./protocols/Aws_query");
var _GetFederationTokenCommand = class _GetFederationTokenCommand extends import_smithy_client.Command.classBuilder().ep({
  ...import_EndpointParameters5.commonParams
}).m(function(Command, cs, config, o) {
  return [
    (0, import_middleware_serde.getSerdePlugin)(config, this.serialize, this.deserialize),
    (0, import_middleware_endpoint.getEndpointPlugin)(config, Command.getEndpointParameterInstructions())
  ];
}).s("AWSSecurityTokenServiceV20110615", "GetFederationToken", {}).n("STSClient", "GetFederationTokenCommand").f(void 0, import_models_02.GetFederationTokenResponseFilterSensitiveLog).ser(import_Aws_query5.se_GetFederationTokenCommand).de(import_Aws_query5.de_GetFederationTokenCommand).build() {
};
__name(_GetFederationTokenCommand, "GetFederationTokenCommand");
var GetFederationTokenCommand = _GetFederationTokenCommand;

// src/commands/GetSessionTokenCommand.ts




var import_EndpointParameters6 = require("./endpoint/EndpointParameters");
var import_models_03 = require("./models/models_0");
var import_Aws_query6 = require("./protocols/Aws_query");
var _GetSessionTokenCommand = class _GetSessionTokenCommand extends import_smithy_client.Command.classBuilder().ep({
  ...import_EndpointParameters6.commonParams
}).m(function(Command, cs, config, o) {
  return [
    (0, import_middleware_serde.getSerdePlugin)(config, this.serialize, this.deserialize),
    (0, import_middleware_endpoint.getEndpointPlugin)(config, Command.getEndpointParameterInstructions())
  ];
}).s("AWSSecurityTokenServiceV20110615", "GetSessionToken", {}).n("STSClient", "GetSessionTokenCommand").f(void 0, import_models_03.GetSessionTokenResponseFilterSensitiveLog).ser(import_Aws_query6.se_GetSessionTokenCommand).de(import_Aws_query6.de_GetSessionTokenCommand).build() {
};
__name(_GetSessionTokenCommand, "GetSessionTokenCommand");
var GetSessionTokenCommand = _GetSessionTokenCommand;

// src/STS.ts
var import_STSClient = require("././STSClient");
var commands = {
  AssumeRoleCommand: import_AssumeRoleCommand.AssumeRoleCommand,
  AssumeRoleWithSAMLCommand,
  AssumeRoleWithWebIdentityCommand: import_AssumeRoleWithWebIdentityCommand.AssumeRoleWithWebIdentityCommand,
  DecodeAuthorizationMessageCommand,
  GetAccessKeyInfoCommand,
  GetCallerIdentityCommand,
  GetFederationTokenCommand,
  GetSessionTokenCommand
};
var _STS = class _STS extends import_STSClient.STSClient {
};
__name(_STS, "STS");
var STS = _STS;
(0, import_smithy_client.createAggregatedClient)(commands, STS);

// src/index.ts
var import_EndpointParameters7 = require("./endpoint/EndpointParameters");
var import_runtimeExtensions = require("././runtimeExtensions");

// src/commands/index.ts
var commands_exports = {};
__export(commands_exports, {
  AssumeRoleWithSAMLCommand: () => AssumeRoleWithSAMLCommand,
  DecodeAuthorizationMessageCommand: () => DecodeAuthorizationMessageCommand,
  GetAccessKeyInfoCommand: () => GetAccessKeyInfoCommand,
  GetCallerIdentityCommand: () => GetCallerIdentityCommand,
  GetFederationTokenCommand: () => GetFederationTokenCommand,
  GetSessionTokenCommand: () => GetSessionTokenCommand
});
__reExport(commands_exports, require("./commands/AssumeRoleCommand"));
__reExport(commands_exports, require("./commands/AssumeRoleWithWebIdentityCommand"));

// src/index.ts
__reExport(src_exports, commands_exports, module.exports);

// src/models/index.ts
var models_exports = {};
__reExport(models_exports, require("./models/models_0"));

// src/index.ts
__reExport(src_exports, models_exports, module.exports);
var import_util_endpoints = require("@aws-sdk/util-endpoints");

// src/defaultRoleAssumers.ts
var import_defaultStsRoleAssumers = require("././defaultStsRoleAssumers");
var import_STSClient2 = require("././STSClient");
var getCustomizableStsClientCtor = /* @__PURE__ */ __name((baseCtor, customizations) => {
  var _a;
  if (!customizations)
    return baseCtor;
  else
    return _a = class extends baseCtor {
      constructor(config) {
        super(config);
        for (const customization of customizations) {
          this.middlewareStack.use(customization);
        }
      }
    }, __name(_a, "CustomizableSTSClient"), _a;
}, "getCustomizableStsClientCtor");
var getDefaultRoleAssumer = /* @__PURE__ */ __name((stsOptions = {}, stsPlugins) => (0, import_defaultStsRoleAssumers.getDefaultRoleAssumer)(stsOptions, getCustomizableStsClientCtor(import_STSClient2.STSClient, stsPlugins)), "getDefaultRoleAssumer");
var getDefaultRoleAssumerWithWebIdentity = /* @__PURE__ */ __name((stsOptions = {}, stsPlugins) => (0, import_defaultStsRoleAssumers.getDefaultRoleAssumerWithWebIdentity)(stsOptions, getCustomizableStsClientCtor(import_STSClient2.STSClient, stsPlugins)), "getDefaultRoleAssumerWithWebIdentity");
var decorateDefaultCredentialProvider = /* @__PURE__ */ __name((provider) => (input) => provider({
  roleAssumer: getDefaultRoleAssumer(input),
  roleAssumerWithWebIdentity: getDefaultRoleAssumerWithWebIdentity(input),
  ...input
}), "decorateDefaultCredentialProvider");

// src/index.ts
var import_STSServiceException = require("./models/STSServiceException");
// Annotate the CommonJS export names for ESM import in node:

0 && (module.exports = {
  AssumeRoleWithSAMLCommand,
  ClientInputEndpointParameters,
  DecodeAuthorizationMessageCommand,
  GetAccessKeyInfoCommand,
  GetCallerIdentityCommand,
  GetFederationTokenCommand,
  GetSessionTokenCommand,
  RuntimeExtension,
  STS,
  STSServiceException,
  decorateDefaultCredentialProvider,
  getDefaultRoleAssumer,
  getDefaultRoleAssumerWithWebIdentity,
  __Client,
  STSClient,
  $Command,
  AssumeRoleCommand,
  AssumeRoleWithWebIdentityCommand,
  ExpiredTokenException,
  MalformedPolicyDocumentException,
  PackedPolicyTooLargeException,
  RegionDisabledException,
  IDPRejectedClaimException,
  InvalidIdentityTokenException,
  IDPCommunicationErrorException,
  InvalidAuthorizationMessageException,
  CredentialsFilterSensitiveLog,
  AssumeRoleResponseFilterSensitiveLog,
  AssumeRoleWithSAMLRequestFilterSensitiveLog,
  AssumeRoleWithSAMLResponseFilterSensitiveLog,
  AssumeRoleWithWebIdentityRequestFilterSensitiveLog,
  AssumeRoleWithWebIdentityResponseFilterSensitiveLog,
  GetFederationTokenResponseFilterSensitiveLog,
  GetSessionTokenResponseFilterSensitiveLog
});

