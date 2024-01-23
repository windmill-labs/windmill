import { AssumeRoleWithWebIdentityParams } from "@aws-sdk/credential-provider-web-identity";
import { SourceProfileInit } from "@smithy/shared-ini-file-loader";
import {
  AwsCredentialIdentity,
  AwsCredentialIdentityProvider,
} from "@smithy/types";
import { AssumeRoleParams } from "./resolveAssumeRoleCredentials";
export interface FromIniInit extends SourceProfileInit {
  mfaCodeProvider?: (mfaSerial: string) => Promise<string>;
  roleAssumer?: (
    sourceCreds: AwsCredentialIdentity,
    params: AssumeRoleParams
  ) => Promise<AwsCredentialIdentity>;
  roleAssumerWithWebIdentity?: (
    params: AssumeRoleWithWebIdentityParams
  ) => Promise<AwsCredentialIdentity>;
}
export declare const fromIni: (
  init?: FromIniInit
) => AwsCredentialIdentityProvider;
