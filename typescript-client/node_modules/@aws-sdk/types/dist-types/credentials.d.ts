import { AwsCredentialIdentity } from "./identity";
import { Provider } from "./util";
/**
 * @public
 *
 * An object representing temporary or permanent AWS credentials.
 *
 * @deprecated Use {@link AwsCredentialIdentity}
 */
export interface Credentials extends AwsCredentialIdentity {
}
/**
 * @public
 *
 * @deprecated Use {@link AwsCredentialIdentityProvider}
 */
export type CredentialProvider = Provider<Credentials>;
