import type { ApiRequestOptions } from './ApiRequestOptions';
type Headers = Record<string, string>;
type Middleware<T> = (value: T) => T | Promise<T>;
type Resolver<T> = (options: ApiRequestOptions) => Promise<T>;
export declare class Interceptors<T> {
    _fns: Middleware<T>[];
    constructor();
    eject(fn: Middleware<T>): void;
    use(fn: Middleware<T>): void;
}
export type OpenAPIConfig = {
    BASE: string;
    CREDENTIALS: 'include' | 'omit' | 'same-origin';
    ENCODE_PATH?: ((path: string) => string) | undefined;
    HEADERS?: Headers | Resolver<Headers> | undefined;
    PASSWORD?: string | Resolver<string> | undefined;
    TOKEN?: string | Resolver<string> | undefined;
    USERNAME?: string | Resolver<string> | undefined;
    VERSION: string;
    WITH_CREDENTIALS: boolean;
    interceptors: {
        request: Interceptors<RequestInit>;
        response: Interceptors<Response>;
    };
};
export declare const OpenAPI: OpenAPIConfig;
export {};
