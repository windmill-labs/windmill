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

// Like `info` but written to stderr, for diagnostics (e.g. the workspace-profile
// banner printed on every command) that must not pollute stdout when a command's
// data output is piped or redirected (e.g. `wmill variable get path > file`).
export function infoStderr(msg: unknown) {
  if (silentMode) return;
  console.error(`\x1b[34m${String(msg)}\x1b[39m`);
}

export function warn(msg: unknown) {
  if (silentMode) return;
  console.log(`\x1b[33m${String(msg)}\x1b[39m`);
}

// Like `warn` but written to stderr; see `infoStderr` for why diagnostics must
// not land on stdout.
export function warnStderr(msg: unknown) {
  if (silentMode) return;
  console.error(`\x1b[33m${String(msg)}\x1b[39m`);
}

export function error(msg: unknown) {
  console.error(`\x1b[31m${String(msg)}\x1b[39m`);
}
