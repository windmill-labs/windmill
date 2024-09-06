export function createLogger({ spinner, verbose } = {}) {
    function write(type, ...args) {
        spinner?.stop();
        console[type](...args);
        spinner?.start();
    }
    return {
        log: (...args) => {
            verbose && write("log", ...args);
        },
        info: (...args) => write("info", ...args),
        warn: (...args) => write("warn", ...args),
        error: (...args) => write("error", ...args),
    };
}
