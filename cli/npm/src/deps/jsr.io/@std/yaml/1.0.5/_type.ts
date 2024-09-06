// Ported from js-yaml v3.13.1:
// https://github.com/nodeca/js-yaml/commit/665aadda42349dcae869f12040d9b10ef18d12da
// Copyright 2011-2015 by Vitaly Puzrin. All rights reserved. MIT license.
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.

export type KindType = "sequence" | "scalar" | "mapping";
/**
 * The style variation for `styles` option of {@linkcode stringify}
 */
export type StyleVariant =
  | "lowercase"
  | "uppercase"
  | "camelcase"
  | "decimal"
  | "binary"
  | "octal"
  | "hexadecimal";

export type RepresentFn<D> = (data: D, style?: StyleVariant) => string;

// deno-lint-ignore no-explicit-any
export interface Type<K extends KindType, D = any> {
  tag: string;
  kind: K;
  predicate?: (data: unknown) => data is D;
  represent?: RepresentFn<D> | Record<string, RepresentFn<D>>;
  defaultStyle?: StyleVariant;
  // deno-lint-ignore no-explicit-any
  resolve: (data: any) => boolean;
  // deno-lint-ignore no-explicit-any
  construct: (data: any) => D;
}
