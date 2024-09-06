// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
import { assertPath } from "../_common/assert_path.js";
import { normalize } from "./normalize.js";
import { fromFileUrl } from "./from_file_url.js";
export function join(path, ...paths) {
    if (path === undefined)
        return ".";
    path = path instanceof URL ? fromFileUrl(path) : path;
    paths = path ? [path, ...paths] : paths;
    paths.forEach((path) => assertPath(path));
    const joined = paths.filter((path) => path.length > 0).join("/");
    return joined === "" ? "." : normalize(joined);
}
