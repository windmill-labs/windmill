// Ported from js-yaml v3.13.1:
// https://github.com/nodeca/js-yaml/commit/665aadda42349dcae869f12040d9b10ef18d12da
// Copyright 2011-2015 by Vitaly Puzrin. All rights reserved. MIT license.
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
import { isPlainObject } from "../_utils.js";
function resolveYamlPairs(data) {
    if (data === null)
        return true;
    return data.every((it) => isPlainObject(it) && Object.keys(it).length === 1);
}
export const pairs = {
    tag: "tag:yaml.org,2002:pairs",
    construct(data) {
        // Converts an array of objects into an array of key-value pairs.
        return data?.flatMap(Object.entries) ?? [];
    },
    kind: "sequence",
    resolve: resolveYamlPairs,
};
