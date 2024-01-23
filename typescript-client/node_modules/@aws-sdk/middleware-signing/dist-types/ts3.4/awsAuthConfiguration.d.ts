import { SignatureV4CryptoInit, SignatureV4Init } from "@smithy/signature-v4";
import {
  AuthScheme,
  AwsCredentialIdentity,
  ChecksumConstructor,
  HashConstructor,
  Logger,
  MemoizedProvider,
  Provider,
  RegionInfoProvider,
  RequestSigner,
} from "@smithy/types";
export interface AwsAuthInputConfig {
  credentials?: AwsCredentialIdentity | Provider<AwsCredentialIdentity>;
  signer?:
    | RequestSigner
    | ((authScheme?: AuthScheme) => Promise<RequestSigner>);
  signingEscapePath?: boolean;
  systemClockOffset?: number;
  signingRegion?: string;
  signerConstructor?: new (
    options: SignatureV4Init & SignatureV4CryptoInit
  ) => RequestSigner;
}
export interface SigV4AuthInputConfig {
  credentials?: AwsCredentialIdentity | Provider<AwsCredentialIdentity>;
  signer?:
    | RequestSigner
    | ((authScheme?: AuthScheme) => Promise<RequestSigner>);
  signingEscapePath?: boolean;
  systemClockOffset?: number;
}
interface PreviouslyResolved {
  credentialDefaultProvider: (
    input: any
  ) => MemoizedProvider<AwsCredentialIdentity>;
  region: string | Provider<string>;
  regionInfoProvider?: RegionInfoProvider;
  signingName?: string;
  defaultSigningName?: string;
  serviceId: string;
  sha256: ChecksumConstructor | HashConstructor;
  useFipsEndpoint: Provider<boolean>;
  useDualstackEndpoint: Provider<boolean>;
}
interface SigV4PreviouslyResolved {
  credentialDefaultProvider: (
    input: any
  ) => MemoizedProvider<AwsCredentialIdentity>;
  region: string | Provider<string>;
  signingName: string;
  sha256: ChecksumConstructor | HashConstructor;
  logger?: Logger;
}
export interface AwsAuthResolvedConfig {
  credentials: MemoizedProvider<AwsCredentialIdentity>;
  signer: (authScheme?: AuthScheme) => Promise<RequestSigner>;
  signingEscapePath: boolean;
  systemClockOffset: number;
}
export interface SigV4AuthResolvedConfig extends AwsAuthResolvedConfig {}
export declare const resolveAwsAuthConfig: <T>(
  input: T & AwsAuthInputConfig & PreviouslyResolved
) => T & AwsAuthResolvedConfig;
export declare const resolveSigV4AuthConfig: <T>(
  input: T & SigV4AuthInputConfig & SigV4PreviouslyResolved
) => T & SigV4AuthResolvedConfig;
export {};
