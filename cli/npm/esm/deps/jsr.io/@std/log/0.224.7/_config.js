// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
import { ConsoleHandler } from "./console_handler.js";
export const DEFAULT_LEVEL = "INFO";
export const DEFAULT_CONFIG = {
    handlers: {
        default: new ConsoleHandler(DEFAULT_LEVEL),
    },
    loggers: {
        default: {
            level: DEFAULT_LEVEL,
            handlers: ["default"],
        },
    },
};
