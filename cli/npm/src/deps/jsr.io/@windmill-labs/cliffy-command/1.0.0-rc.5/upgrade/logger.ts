import type { Spinner } from "./spinner.js";

export interface Logger {
  log(...data: Array<unknown>): void;
  info(...data: Array<unknown>): void;
  warn(...data: Array<unknown>): void;
  error(...data: Array<unknown>): void;
}

export interface LoggerOptions {
  spinner?: Spinner;
  verbose?: boolean;
}

export function createLogger({ spinner, verbose }: LoggerOptions = {}): Logger {
  function write(
    type: "log" | "info" | "warn" | "error",
    ...args: Array<unknown>
  ): void {
    spinner?.stop();
    console[type](...args);
    spinner?.start();
  }

  return {
    log: (...args: Array<unknown>): void => {
      verbose && write("log", ...args);
    },
    info: (...args: Array<unknown>): void => write("info", ...args),
    warn: (...args: Array<unknown>): void => write("warn", ...args),
    error: (...args: Array<unknown>): void => write("error", ...args),
  };
}
