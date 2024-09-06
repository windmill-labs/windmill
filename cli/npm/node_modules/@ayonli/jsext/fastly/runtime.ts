// @ts-ignore
import { env as _env } from "fastly:env";

export * from "../runtime.ts";
export { default } from "../runtime.ts";

export function env(): { [name: string]: string; };
export function env(name: string): string | undefined;
export function env(name: string, value: string): undefined;
export function env(obj: object): void;
export function env(
    name: string | undefined | object = undefined,
    value: string | undefined = undefined
): any {
    if (typeof name === "object") {
        throw new Error("Not implemented");
    } else if (value !== undefined) {
        throw new Error("Cannot modify environment variables in the worker");
    } else if (name === undefined) {
        return {
            "FASTLY_CACHE_GENERATION": _env("FASTLY_CACHE_GENERATION") ?? "",
            "FASTLY_CUSTOMER_ID": _env("FASTLY_CUSTOMER_ID") ?? "",
            "FASTLY_HOSTNAME": _env("FASTLY_HOSTNAME") ?? "",
            "FASTLY_POP": _env("FASTLY_POP") ?? "",
            "FASTLY_REGION": _env("FASTLY_REGION") ?? "",
            "FASTLY_SERVICE_ID": _env("FASTLY_SERVICE_ID") ?? "",
            "FASTLY_SERVICE_VERSION": _env("FASTLY_SERVICE_VERSION") ?? "",
            "FASTLY_TRACE_ID": _env("FASTLY_TRACE_ID") ?? "",
        };
    }

    return _env(name);
}
