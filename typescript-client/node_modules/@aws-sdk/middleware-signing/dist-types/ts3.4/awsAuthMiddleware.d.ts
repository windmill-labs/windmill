import {
  FinalizeRequestMiddleware,
  Pluggable,
  RelativeMiddlewareOptions,
} from "@smithy/types";
import { AwsAuthResolvedConfig } from "./awsAuthConfiguration";
export declare const awsAuthMiddleware: <
  Input extends object,
  Output extends object
>(
  options: AwsAuthResolvedConfig
) => FinalizeRequestMiddleware<Input, Output>;
export declare const awsAuthMiddlewareOptions: RelativeMiddlewareOptions;
export declare const getAwsAuthPlugin: (
  options: AwsAuthResolvedConfig
) => Pluggable<any, any>;
export declare const getSigV4AuthPlugin: (
  options: AwsAuthResolvedConfig
) => Pluggable<any, any>;
