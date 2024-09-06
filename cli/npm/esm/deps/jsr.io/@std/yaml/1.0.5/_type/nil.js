// Ported from js-yaml v3.13.1:
// https://github.com/nodeca/js-yaml/commit/665aadda42349dcae869f12040d9b10ef18d12da
// Copyright 2011-2015 by Vitaly Puzrin. All rights reserved. MIT license.
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
export const nil = {
    tag: "tag:yaml.org,2002:null",
    kind: "scalar",
    defaultStyle: "lowercase",
    predicate: (object) => object === null,
    construct: () => null,
    resolve: (data) => {
        return (data === "~" ||
            data === "null" ||
            data === "Null" ||
            data === "NULL");
    },
    represent: {
        lowercase: () => "null",
        uppercase: () => "NULL",
        camelcase: () => "Null",
    },
};
