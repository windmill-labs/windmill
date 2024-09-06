// deno-lint-ignore-file no-explicit-any
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.

// Check Deno, then the remaining runtimes (e.g. Node, Bun and the browser)
import * as dntShim from "../../../../../_dnt.shims.js";

export const isWindows: boolean =
  (dntShim.dntGlobalThis as any).Deno?.build.os === "windows" ||
  (dntShim.dntGlobalThis as any).navigator?.platform?.startsWith("Win") ||
  (dntShim.dntGlobalThis as any).process?.platform?.startsWith("win") ||
  false;
