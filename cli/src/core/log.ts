let logLevel: "DEBUG" | "INFO" | "WARN" | "ERROR" = "INFO";
let silentMode = false;

const levels = { DEBUG: 0, INFO: 1, WARN: 2, ERROR: 3 };

export function setup(level: "DEBUG" | "INFO" | "WARN" | "ERROR") {
  logLevel = level;
}

export function setSilent(silent: boolean) {
  silentMode = silent;
}

export function debug(msg: unknown) {
  if (levels[logLevel] <= levels.DEBUG)
    console.log(`\x1b[90m${String(msg)}\x1b[39m`);
}

export function info(msg: unknown) {
  if (silentMode) return;
  console.log(`\x1b[34m${String(msg)}\x1b[39m`);
}

export function warn(msg: unknown) {
  if (silentMode) return;
  console.log(`\x1b[33m${String(msg)}\x1b[39m`);
}

export function error(msg: unknown) {
  console.error(`\x1b[31m${String(msg)}\x1b[39m`);
}
