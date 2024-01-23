import { AwsCredentialIdentity } from "./identity";
import { Provider } from "./util";
export interface Credentials extends AwsCredentialIdentity {}
export type CredentialProvider = Provider<Credentials>;
