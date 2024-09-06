// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.

import { getLogger } from "./get_logger.js";
import type { GenericFunction } from "./logger.js";

/** Log with error level, using default logger. */
export function error<T>(msg: () => T, ...args: unknown[]): T | undefined;
export function error<T>(
  msg: T extends GenericFunction ? never : T,
  ...args: unknown[]
): T;
export function error<T>(
  msg: (T extends GenericFunction ? never : T) | (() => T),
  ...args: unknown[]
): T | undefined {
  // Assist TS compiler with pass-through generic type
  if (msg instanceof Function) {
    return getLogger("default").error(msg, ...args);
  }
  return getLogger("default").error(msg, ...args);
}
