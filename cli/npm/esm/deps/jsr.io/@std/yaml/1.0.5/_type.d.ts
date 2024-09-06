export type KindType = "sequence" | "scalar" | "mapping";
/**
 * The style variation for `styles` option of {@linkcode stringify}
 */
export type StyleVariant = "lowercase" | "uppercase" | "camelcase" | "decimal" | "binary" | "octal" | "hexadecimal";
export type RepresentFn<D> = (data: D, style?: StyleVariant) => string;
export interface Type<K extends KindType, D = any> {
    tag: string;
    kind: K;
    predicate?: (data: unknown) => data is D;
    represent?: RepresentFn<D> | Record<string, RepresentFn<D>>;
    defaultStyle?: StyleVariant;
    resolve: (data: any) => boolean;
    construct: (data: any) => D;
}
//# sourceMappingURL=_type.d.ts.map