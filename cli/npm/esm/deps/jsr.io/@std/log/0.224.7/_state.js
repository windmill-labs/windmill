// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
import { DEFAULT_CONFIG } from "./_config.js";
export const state = {
    handlers: new Map(),
    loggers: new Map(),
    config: DEFAULT_CONFIG,
};
