// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
import { type LevelName, LogLevels } from "./levels.js";
import type { LogRecord } from "./logger.js";
import { blue, bold, red, yellow } from "../../fmt/1.0.2/colors.js";
import { BaseHandler, type BaseHandlerOptions } from "./base_handler.js";

export interface ConsoleHandlerOptions extends BaseHandlerOptions {
  useColors?: boolean;
}

/**
 * This is the default logger. It will output color coded log messages to the
 * console via `console.log()`.
 */
export class ConsoleHandler extends BaseHandler {
  #useColors?: boolean;

  constructor(levelName: LevelName, options: ConsoleHandlerOptions = {}) {
    super(levelName, options);
    this.#useColors = options.useColors ?? true;
  }

  override format(logRecord: LogRecord): string {
    let msg = super.format(logRecord);

    if (this.#useColors) {
      msg = this.applyColors(msg, logRecord.level);
    }

    return msg;
  }

  applyColors(msg: string, level: number): string {
    switch (level) {
      case LogLevels.INFO:
        msg = blue(msg);
        break;
      case LogLevels.WARN:
        msg = yellow(msg);
        break;
      case LogLevels.ERROR:
        msg = red(msg);
        break;
      case LogLevels.CRITICAL:
        msg = bold(red(msg));
        break;
      default:
        break;
    }

    return msg;
  }

  log(msg: string) {
    console.log(msg);
  }
}
