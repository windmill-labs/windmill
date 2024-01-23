import { Pluggable } from "@smithy/types";
import { PreviouslyResolved } from "./configuration";
import { FlexibleChecksumsRequestMiddlewareConfig } from "./flexibleChecksumsMiddleware";
import { FlexibleChecksumsResponseMiddlewareConfig } from "./flexibleChecksumsResponseMiddleware";
export interface FlexibleChecksumsMiddlewareConfig
  extends FlexibleChecksumsRequestMiddlewareConfig,
    FlexibleChecksumsResponseMiddlewareConfig {}
export declare const getFlexibleChecksumsPlugin: (
  config: PreviouslyResolved,
  middlewareConfig: FlexibleChecksumsMiddlewareConfig
) => Pluggable<any, any>;
