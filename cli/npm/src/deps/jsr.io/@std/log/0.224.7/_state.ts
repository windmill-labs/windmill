// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

import type { BaseHandler } from "./base_handler.js";
import { DEFAULT_CONFIG } from "./_config.js";
import type { Logger } from "./logger.js";

export const state = {
  handlers: new Map<string, BaseHandler>(),
  loggers: new Map<string, Logger>(),
  config: DEFAULT_CONFIG,
};
