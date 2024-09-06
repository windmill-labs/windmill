// Ported from js-yaml v3.13.1:
// https://github.com/nodeca/js-yaml/commit/665aadda42349dcae869f12040d9b10ef18d12da
// Copyright 2011-2015 by Vitaly Puzrin. All rights reserved. MIT license.
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

import type { Type } from "../_type.js";

export const nil: Type<"scalar", null> = {
  tag: "tag:yaml.org,2002:null",
  kind: "scalar",
  defaultStyle: "lowercase",
  predicate: (object: unknown): object is null => object === null,
  construct: () => null,
  resolve: (data: string): boolean => {
    return (
      data === "~" ||
      data === "null" ||
      data === "Null" ||
      data === "NULL"
    );
  },
  represent: {
    lowercase: (): string => "null",
    uppercase: (): string => "NULL",
    camelcase: (): string => "Null",
  },
};
