import { SignatureV4CryptoInit, SignatureV4Init } from "@smithy/signature-v4";
import { AuthScheme, AwsCredentialIdentity, ChecksumConstructor, HashConstructor, Logger, MemoizedProvider, Provider, RegionInfoProvider, RequestSigner } from "@smithy/types";
/**
 * @public
 */
export interface AwsAuthInputConfig {
    /**
     * The credentials used to sign requests.
     */
    credentials?: AwsCredentialIdentity | Provider<AwsCredentialIdentity>;
    /**
     * The signer to use when signing requests.
     */
    signer?: RequestSigner | ((authScheme?: AuthScheme) => Promise<RequestSigner>);
    /**
     * Whether to escape request path when signing the request.
     */
    signingEscapePath?: boolean;
    /**
     * An offset value in milliseconds to apply to all signing times.
     */
    systemClockOffset?: number;
    /**
     * The region where you want to sign your request against. This
     * can be different to the region in the endpoint.
     */
    signingRegion?: string;
    /**
     * The injectable SigV4-compatible signer class constructor. If not supplied,
     * regular SignatureV4 constructor will be used.
     *
     * @internal
     */
    signerConstructor?: new (options: SignatureV4Init & SignatureV4CryptoInit) => RequestSigner;
}
/**
 * @public
 */
export interface SigV4AuthInputConfig {
    /**
     * The credentials used to sign requests.
     */
    credentials?: AwsCredentialIdentity | Provider<AwsCredentialIdentity>;
    /**
     * The signer to use when signing requests.
     */
    signer?: RequestSigner | ((authScheme?: AuthScheme) => Promise<RequestSigner>);
    /**
     * Whether to escape request path when signing the request.
     */
    signingEscapePath?: boolean;
    /**
     * An offset value in milliseconds to apply to all signing times.
     */
    systemClockOffset?: number;
}
interface PreviouslyResolved {
    credentialDefaultProvider: (input: any) => MemoizedProvider<AwsCredentialIdentity>;
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
    credentialDefaultProvider: (input: any) => MemoizedProvider<AwsCredentialIdentity>;
    region: string | Provider<string>;
    signingName: string;
    sha256: ChecksumConstructor | HashConstructor;
    logger?: Logger;
}
export interface AwsAuthResolvedConfig {
    /**
     * Resolved value for input config {@link AwsAuthInputConfig.credentials}
     * This provider MAY memoize the loaded credentials for certain period.
     * See {@link MemoizedProvider} for more information.
     */
    credentials: MemoizedProvider<AwsCredentialIdentity>;
    /**
     * Resolved value for input config {@link AwsAuthInputConfig.signer}
     */
    signer: (authScheme?: AuthScheme) => Promise<RequestSigner>;
    /**
     * Resolved value for input config {@link AwsAuthInputConfig.signingEscapePath}
     */
    signingEscapePath: boolean;
    /**
     * Resolved value for input config {@link AwsAuthInputConfig.systemClockOffset}
     */
    systemClockOffset: number;
}
export interface SigV4AuthResolvedConfig extends AwsAuthResolvedConfig {
}
export declare const resolveAwsAuthConfig: <T>(input: T & AwsAuthInputConfig & PreviouslyResolved) => T & AwsAuthResolvedConfig;
export declare const resolveSigV4AuthConfig: <T>(input: T & SigV4AuthInputConfig & SigV4PreviouslyResolved) => T & SigV4AuthResolvedConfig;
export {};
