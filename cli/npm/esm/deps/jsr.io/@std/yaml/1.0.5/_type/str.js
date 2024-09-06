// Ported from js-yaml v3.13.1:
// https://github.com/nodeca/js-yaml/commit/665aadda42349dcae869f12040d9b10ef18d12da
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
export const str = {
    tag: "tag:yaml.org,2002:str",
    kind: "scalar",
    resolve: () => true,
    construct: (data) => data !== null ? data : "",
};
