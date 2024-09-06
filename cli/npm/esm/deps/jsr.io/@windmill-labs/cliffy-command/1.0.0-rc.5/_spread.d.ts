/** Merge types of two objects. */
export type Spread<TTarget, TSource> = TTarget extends void ? TSource : TSource extends void ? TTarget : Omit<TTarget, keyof TSource> & Omit<TSource, keyof TTarget> & SpreadRequiredProperties<TTarget, TSource, RequiredKeys<TSource> & keyof TTarget> & SpreadRequiredProperties<TTarget, TSource, RequiredKeys<TTarget> & keyof TSource> & SpreadOptionalProperties<TTarget, TSource, OptionalKeys<TTarget> & OptionalKeys<TSource>>;
type RequiredKeys<TRecord> = {
    [Key in keyof TRecord]-?: {} extends Pick<TRecord, Key> ? never : Key;
}[keyof TRecord];
type OptionalKeys<TRecord> = {
    [Key in keyof TRecord]-?: {} extends Pick<TRecord, Key> ? Key : never;
}[keyof TRecord];
type SpreadRequiredProperties<TTarget, TSource, TKeys extends keyof TTarget & keyof TSource> = {
    [Key in TKeys]: Exclude<TTarget[Key], undefined> | Exclude<TSource[Key], undefined>;
};
type SpreadOptionalProperties<TTarget, TSource, TKeys extends keyof TTarget & keyof TSource> = {
    [Key in TKeys]?: TTarget[Key] | TSource[Key];
};
export {};
//# sourceMappingURL=_spread.d.ts.map