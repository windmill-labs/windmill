// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.

export function assertArg(url: URL | string) {
  url = url instanceof URL ? url : new URL(url);
  if (url.protocol !== "file:") {
    throw new TypeError(
      `URL must be a file URL: received "${url.protocol}"`,
    );
  }
  return url;
}
