import { HttpHandlerOptions, RequestHandler } from "@smithy/types";
import { HttpRequest } from "./httpRequest";
import { HttpResponse } from "./httpResponse";
/**
 * @internal
 */
export type HttpHandler<HttpHandlerConfig extends object = {}> = RequestHandler<HttpRequest, HttpResponse, HttpHandlerOptions> & {
    /**
     * @internal
     * @param key
     * @param value
     */
    updateHttpClientConfig(key: keyof HttpHandlerConfig, value: HttpHandlerConfig[typeof key]): void;
    /**
     * @internal
     */
    httpHandlerConfigs(): HttpHandlerConfig;
};
