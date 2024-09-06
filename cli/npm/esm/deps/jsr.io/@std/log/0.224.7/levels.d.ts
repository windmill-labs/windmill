/**
 * Use this to retrieve the numeric log level by it's associated name.
 * Defaults to INFO.
 */
export declare const LogLevels: {
    readonly NOTSET: 0;
    readonly DEBUG: 10;
    readonly INFO: 20;
    readonly WARN: 30;
    readonly ERROR: 40;
    readonly CRITICAL: 50;
};
/** Union of valid log levels */
export type LogLevel = typeof LogLevels[LevelName];
/** Union of valid log level names */
export type LevelName = Exclude<keyof typeof LogLevels, number>;
/** Permitted log level names */
export declare const LogLevelNames: LevelName[];
/**
 * Returns the numeric log level associated with the passed,
 * stringy log level name.
 */
export declare function getLevelByName(name: LevelName): LogLevel;
/** Returns the stringy log level name provided the numeric log level. */
export declare function getLevelName(level: LogLevel): LevelName;
//# sourceMappingURL=levels.d.ts.map