import { HttpHandler, HttpRequest, HttpResponse } from "@smithy/protocol-http";
import { NodeHttpHandlerOptions } from "@smithy/types";
import { HttpHandlerOptions, Provider } from "@smithy/types";
export { NodeHttpHandlerOptions };
export declare const DEFAULT_REQUEST_TIMEOUT = 0;
export declare class NodeHttpHandler implements HttpHandler<NodeHttpHandlerOptions> {
    private config?;
    private configProvider;
    readonly metadata: {
        handlerProtocol: string;
    };
    /**
     * @returns the input if it is an HttpHandler of any class,
     * or instantiates a new instance of this handler.
     */
    static create(instanceOrOptions?: HttpHandler<any> | NodeHttpHandlerOptions | Provider<NodeHttpHandlerOptions | void>): NodeHttpHandler | HttpHandler<any>;
    constructor(options?: NodeHttpHandlerOptions | Provider<NodeHttpHandlerOptions | void>);
    private resolveDefaultConfig;
    destroy(): void;
    handle(request: HttpRequest, { abortSignal }?: HttpHandlerOptions): Promise<{
        response: HttpResponse;
    }>;
    updateHttpClientConfig(key: keyof NodeHttpHandlerOptions, value: NodeHttpHandlerOptions[typeof key]): void;
    httpHandlerConfigs(): NodeHttpHandlerOptions;
}
