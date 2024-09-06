// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
// Keep this up-to-date with Deno.build.os
import * as dntShim from "../../../../_dnt.shims.js";
export const osType = (() => {
    // deno-lint-ignore no-explicit-any
    const { Deno } = dntShim.dntGlobalThis;
    if (typeof Deno?.build?.os === "string") {
        return Deno.build.os;
    }
    // deno-lint-ignore no-explicit-any
    const { navigator } = dntShim.dntGlobalThis;
    if (navigator?.appVersion?.includes?.("Win")) {
        return "windows";
    }
    return "linux";
})();
export const isWindows = osType === "windows";
