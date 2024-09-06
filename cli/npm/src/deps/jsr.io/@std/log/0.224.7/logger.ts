// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
import { getLevelByName, getLevelName, LogLevels } from "./levels.js";
import type { LevelName, LogLevel } from "./levels.js";
import type { BaseHandler } from "./base_handler.js";

// deno-lint-ignore no-explicit-any
export type GenericFunction = (...args: any[]) => any;

export interface LogRecordOptions {
  msg: string;
  args: unknown[];
  level: LogLevel;
  loggerName: string;
}

export class LoggerConfig {
  level?: LevelName;
  handlers?: string[];
}

export interface LogConfig {
  handlers?: {
    [name: string]: BaseHandler;
  };
  loggers?: {
    [name: string]: LoggerConfig;
  };
}

/**
 * An object that encapsulates provided message and arguments as well some
 * metadata that can be later used when formatting a message.
 */
export class LogRecord {
  readonly msg: string;
  #args: unknown[];
  #datetime: Date;
  readonly level: number;
  readonly levelName: string;
  readonly loggerName: string;

  constructor(options: LogRecordOptions) {
    this.msg = options.msg;
    this.#args = [...options.args];
    this.level = options.level;
    this.loggerName = options.loggerName;
    this.#datetime = new Date();
    this.levelName = getLevelName(options.level);
  }
  get args(): unknown[] {
    return [...this.#args];
  }
  get datetime(): Date {
    return new Date(this.#datetime.getTime());
  }
}

export interface LoggerOptions {
  handlers?: BaseHandler[];
}

export class Logger {
  #level: LogLevel;
  handlers: BaseHandler[];
  readonly #loggerName: string;

  constructor(
    loggerName: string,
    levelName: LevelName,
    options: LoggerOptions = {},
  ) {
    this.#loggerName = loggerName;
    this.#level = getLevelByName(levelName);
    this.handlers = options.handlers ?? [];
  }

  /** Use this to retrieve the current numeric log level. */
  get level(): LogLevel {
    return this.#level;
  }

  /** Use this to set the numeric log level. */
  set level(level: LogLevel) {
    try {
      this.#level = getLevelByName(getLevelName(level));
    } catch (_) {
      throw new TypeError(`Invalid log level: ${level}`);
    }
  }

  get levelName(): LevelName {
    return getLevelName(this.#level);
  }
  set levelName(levelName: LevelName) {
    this.#level = getLevelByName(levelName);
  }

  get loggerName(): string {
    return this.#loggerName;
  }

  /**
   * If the level of the logger is greater than the level to log, then nothing
   * is logged, otherwise a log record is passed to each log handler.  `msg` data
   * passed in is returned.  If a function is passed in, it is only evaluated
   * if the msg will be logged and the return value will be the result of the
   * function, not the function itself, unless the function isn't called, in which
   * case undefined is returned.  All types are coerced to strings for logging.
   */
  #log<T>(
    level: LogLevel,
    msg: (T extends GenericFunction ? never : T) | (() => T),
    ...args: unknown[]
  ): T | undefined {
    if (this.level > level) {
      return msg instanceof Function ? undefined : msg;
    }

    let fnResult: T | undefined;
    let logMessage: string;
    if (msg instanceof Function) {
      fnResult = msg();
      logMessage = this.asString(fnResult);
    } else {
      logMessage = this.asString(msg);
    }
    const record: LogRecord = new LogRecord({
      msg: logMessage,
      args: args,
      level: level,
      loggerName: this.loggerName,
    });

    this.handlers.forEach((handler) => {
      handler.handle(record);
    });

    return msg instanceof Function ? fnResult : msg;
  }

  asString(data: unknown, isProperty = false): string {
    if (typeof data === "string") {
      if (isProperty) return `"${data}"`;
      return data;
    } else if (
      data === null ||
      typeof data === "number" ||
      typeof data === "bigint" ||
      typeof data === "boolean" ||
      typeof data === "undefined" ||
      typeof data === "symbol"
    ) {
      return String(data);
    } else if (data instanceof Error) {
      return data.stack!;
    } else if (typeof data === "object") {
      return `{${
        Object.entries(data)
          .map(([k, v]) => `"${k}":${this.asString(v, true)}`)
          .join(",")
      }}`;
    }
    return "undefined";
  }

  debug<T>(msg: () => T, ...args: unknown[]): T | undefined;
  debug<T>(msg: T extends GenericFunction ? never : T, ...args: unknown[]): T;
  debug<T>(
    msg: (T extends GenericFunction ? never : T) | (() => T),
    ...args: unknown[]
  ): T | undefined {
    return this.#log(LogLevels.DEBUG, msg, ...args);
  }

  info<T>(msg: () => T, ...args: unknown[]): T | undefined;
  info<T>(msg: T extends GenericFunction ? never : T, ...args: unknown[]): T;
  info<T>(
    msg: (T extends GenericFunction ? never : T) | (() => T),
    ...args: unknown[]
  ): T | undefined {
    return this.#log(LogLevels.INFO, msg, ...args);
  }

  warn<T>(msg: () => T, ...args: unknown[]): T | undefined;
  warn<T>(msg: T extends GenericFunction ? never : T, ...args: unknown[]): T;
  warn<T>(
    msg: (T extends GenericFunction ? never : T) | (() => T),
    ...args: unknown[]
  ): T | undefined {
    return this.#log(LogLevels.WARN, msg, ...args);
  }

  error<T>(msg: () => T, ...args: unknown[]): T | undefined;
  error<T>(msg: T extends GenericFunction ? never : T, ...args: unknown[]): T;
  error<T>(
    msg: (T extends GenericFunction ? never : T) | (() => T),
    ...args: unknown[]
  ): T | undefined {
    return this.#log(LogLevels.ERROR, msg, ...args);
  }

  critical<T>(msg: () => T, ...args: unknown[]): T | undefined;
  critical<T>(
    msg: T extends GenericFunction ? never : T,
    ...args: unknown[]
  ): T;
  critical<T>(
    msg: (T extends GenericFunction ? never : T) | (() => T),
    ...args: unknown[]
  ): T | undefined {
    return this.#log(LogLevels.CRITICAL, msg, ...args);
  }
}
