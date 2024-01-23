import {
  DeserializeMiddleware,
  Encoder,
  Pluggable,
  RelativeMiddlewareOptions,
  StreamCollector,
} from "@smithy/types";
type PreviouslyResolved = {
  streamCollector: StreamCollector;
  utf8Encoder: Encoder;
};
export declare const throw200ExceptionsMiddleware: (
  config: PreviouslyResolved
) => DeserializeMiddleware<any, any>;
export declare const throw200ExceptionsMiddlewareOptions: RelativeMiddlewareOptions;
export declare const getThrow200ExceptionsPlugin: (
  config: PreviouslyResolved
) => Pluggable<any, any>;
export {};
