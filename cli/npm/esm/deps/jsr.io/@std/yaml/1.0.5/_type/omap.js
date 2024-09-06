// Ported from js-yaml v3.13.1:
// https://github.com/nodeca/js-yaml/commit/665aadda42349dcae869f12040d9b10ef18d12da
// Copyright 2011-2015 by Vitaly Puzrin. All rights reserved. MIT license.
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
import { isPlainObject } from "../_utils.js";
function resolveYamlOmap(data) {
    const objectKeys = new Set();
    for (const object of data) {
        if (!isPlainObject(object))
            return false;
        const keys = Object.keys(object);
        if (keys.length !== 1)
            return false;
        for (const key of keys) {
            if (objectKeys.has(key))
                return false;
            objectKeys.add(key);
        }
    }
    return true;
}
export const omap = {
    tag: "tag:yaml.org,2002:omap",
    kind: "sequence",
    resolve: resolveYamlOmap,
    construct(data) {
        return data;
    },
};
