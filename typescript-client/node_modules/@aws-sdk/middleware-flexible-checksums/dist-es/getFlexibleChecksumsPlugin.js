import { flexibleChecksumsMiddleware, flexibleChecksumsMiddlewareOptions, } from "./flexibleChecksumsMiddleware";
import { flexibleChecksumsResponseMiddleware, flexibleChecksumsResponseMiddlewareOptions, } from "./flexibleChecksumsResponseMiddleware";
export const getFlexibleChecksumsPlugin = (config, middlewareConfig) => ({
    applyToStack: (clientStack) => {
        clientStack.add(flexibleChecksumsMiddleware(config, middlewareConfig), flexibleChecksumsMiddlewareOptions);
        clientStack.addRelativeTo(flexibleChecksumsResponseMiddleware(config, middlewareConfig), flexibleChecksumsResponseMiddlewareOptions);
    },
});
