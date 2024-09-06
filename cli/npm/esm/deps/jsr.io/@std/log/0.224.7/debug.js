// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
import { getLogger } from "./get_logger.js";
export function debug(msg, ...args) {
    // Assist TS compiler with pass-through generic type
    if (msg instanceof Function) {
        return getLogger("default").debug(msg, ...args);
    }
    return getLogger("default").debug(msg, ...args);
}
