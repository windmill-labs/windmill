/** Merge types of two objects. */
export type Spread<TTarget, TSource> = TTarget extends void ? TSource
  : TSource extends void ? TTarget
  // Properties in L that don't exist in R.
  :
    & Omit<TTarget, keyof TSource>
    // Properties in R that don't exist in L.
    & Omit<TSource, keyof TTarget>
    // Required properties in R that exist in L.
    & SpreadRequiredProperties<
      TTarget,
      TSource,
      RequiredKeys<TSource> & keyof TTarget
    >
    // Required properties in L that exist in R.
    & SpreadRequiredProperties<
      TTarget,
      TSource,
      RequiredKeys<TTarget> & keyof TSource
    >
    // Optional properties in L and R.
    & SpreadOptionalProperties<
      TTarget,
      TSource,
      OptionalKeys<TTarget> & OptionalKeys<TSource>
    >;

type RequiredKeys<TRecord> = {
  // deno-lint-ignore ban-types
  [Key in keyof TRecord]-?: {} extends Pick<TRecord, Key> ? never : Key;
}[keyof TRecord];

type OptionalKeys<TRecord> = {
  // deno-lint-ignore ban-types
  [Key in keyof TRecord]-?: {} extends Pick<TRecord, Key> ? Key : never;
}[keyof TRecord];

type SpreadRequiredProperties<
  TTarget,
  TSource,
  TKeys extends keyof TTarget & keyof TSource,
> = {
  [Key in TKeys]:
    | Exclude<TTarget[Key], undefined>
    | Exclude<TSource[Key], undefined>;
};

type SpreadOptionalProperties<
  TTarget,
  TSource,
  TKeys extends keyof TTarget & keyof TSource,
> = {
  [Key in TKeys]?: TTarget[Key] | TSource[Key];
};
