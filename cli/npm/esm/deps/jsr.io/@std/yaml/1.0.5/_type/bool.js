// Ported from js-yaml v3.13.1:
// https://github.com/nodeca/js-yaml/commit/665aadda42349dcae869f12040d9b10ef18d12da
// Copyright 2011-2015 by Vitaly Puzrin. All rights reserved. MIT license.
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
const YAML_TRUE_BOOLEANS = ["true", "True", "TRUE"];
const YAML_FALSE_BOOLEANS = ["false", "False", "FALSE"];
const YAML_BOOLEANS = [...YAML_TRUE_BOOLEANS, ...YAML_FALSE_BOOLEANS];
export const bool = {
    tag: "tag:yaml.org,2002:bool",
    kind: "scalar",
    defaultStyle: "lowercase",
    predicate: (value) => typeof value === "boolean" || value instanceof Boolean,
    construct: (data) => YAML_TRUE_BOOLEANS.includes(data),
    resolve: (data) => YAML_BOOLEANS.includes(data),
    represent: {
        // deno-lint-ignore ban-types
        lowercase: (object) => {
            const value = object instanceof Boolean ? object.valueOf() : object;
            return value ? "true" : "false";
        },
        // deno-lint-ignore ban-types
        uppercase: (object) => {
            const value = object instanceof Boolean ? object.valueOf() : object;
            return value ? "TRUE" : "FALSE";
        },
        // deno-lint-ignore ban-types
        camelcase: (object) => {
            const value = object instanceof Boolean ? object.valueOf() : object;
            return value ? "True" : "False";
        },
    },
};
