import { runPolling } from "./poller";
import { validateWaiterOptions } from "./utils";
import { waiterServiceDefaults, WaiterState } from "./waiter";
const abortTimeout = async (abortSignal) => {
    return new Promise((resolve) => {
        abortSignal.onabort = () => resolve({ state: WaiterState.ABORTED });
    });
};
export const createWaiter = async (options, input, acceptorChecks) => {
    const params = {
        ...waiterServiceDefaults,
        ...options,
    };
    validateWaiterOptions(params);
    const exitConditions = [runPolling(params, input, acceptorChecks)];
    if (options.abortController) {
        exitConditions.push(abortTimeout(options.abortController.signal));
    }
    if (options.abortSignal) {
        exitConditions.push(abortTimeout(options.abortSignal));
    }
    return Promise.race(exitConditions);
};
