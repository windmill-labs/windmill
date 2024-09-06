// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
import type { LogRecord } from "./logger.js";

export function jsonFormatter(logRecord: LogRecord): string {
  return JSON.stringify({
    level: logRecord.levelName,
    datetime: logRecord.datetime.getTime(),
    message: logRecord.msg,
    args: flattenArgs(logRecord.args),
  });
}

function flattenArgs(args: unknown[]): unknown {
  if (args.length === 1) {
    return args[0];
  } else if (args.length > 1) {
    return args;
  }
}

export const formatters: {
  jsonFormatter(logRecord: LogRecord): string;
} = {
  jsonFormatter,
};
