// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.

import { assertPath } from "./assert_path.js";

export function assertArgs(from: string, to: string) {
  assertPath(from);
  assertPath(to);
  if (from === to) return "";
}
