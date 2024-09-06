// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.

import { getLogger } from "./get_logger.js";
import type { GenericFunction } from "./logger.js";

/** Log with critical level, using default logger. */
export function critical<T>(msg: () => T, ...args: unknown[]): T | undefined;
export function critical<T>(
  msg: T extends GenericFunction ? never : T,
  ...args: unknown[]
): T;
export function critical<T>(
  msg: (T extends GenericFunction ? never : T) | (() => T),
  ...args: unknown[]
): T | undefined {
  // Assist TS compiler with pass-through generic type
  if (msg instanceof Function) {
    return getLogger("default").critical(msg, ...args);
  }
  return getLogger("default").critical(msg, ...args);
}
