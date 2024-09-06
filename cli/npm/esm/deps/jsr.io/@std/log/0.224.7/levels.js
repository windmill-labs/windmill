// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
/**
 * Use this to retrieve the numeric log level by it's associated name.
 * Defaults to INFO.
 */
export const LogLevels = {
    NOTSET: 0,
    DEBUG: 10,
    INFO: 20,
    WARN: 30,
    ERROR: 40,
    CRITICAL: 50,
};
/** Permitted log level names */
export const LogLevelNames = Object.keys(LogLevels).filter((key) => isNaN(Number(key)));
const byLevel = {
    [LogLevels.NOTSET]: "NOTSET",
    [LogLevels.DEBUG]: "DEBUG",
    [LogLevels.INFO]: "INFO",
    [LogLevels.WARN]: "WARN",
    [LogLevels.ERROR]: "ERROR",
    [LogLevels.CRITICAL]: "CRITICAL",
};
/**
 * Returns the numeric log level associated with the passed,
 * stringy log level name.
 */
export function getLevelByName(name) {
    const level = LogLevels[name];
    if (level !== undefined) {
        return level;
    }
    throw new Error(`Cannot get log level: no level named ${name}`);
}
/** Returns the stringy log level name provided the numeric log level. */
export function getLevelName(level) {
    const levelName = byLevel[level];
    if (levelName) {
        return levelName;
    }
    throw new Error(`Cannot get log level: no name for level: ${level}`);
}
