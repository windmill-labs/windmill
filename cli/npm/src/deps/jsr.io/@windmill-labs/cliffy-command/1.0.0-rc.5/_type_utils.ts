export type TrimLeft<
  TValue extends string,
  TTrimValue extends string | undefined,
> = TValue extends `${TTrimValue}${infer TRest}` ? TRest
  : TValue;

export type TrimRight<TValue extends string, TTrimValue extends string> =
  TValue extends `${infer TRest}${TTrimValue}` ? TRest
    : TValue;

export type Lower<TValue extends string> = TValue extends Uppercase<TValue>
  ? Lowercase<TValue>
  : Uncapitalize<TValue>;

export type CamelCase<TValue extends string> = TValue extends
  `${infer TPart}_${infer TRest}`
  ? `${Lower<TPart>}${Capitalize<CamelCase<TRest>>}`
  : TValue extends `${infer TPart}-${infer TRest}`
    ? `${Lower<TPart>}${Capitalize<CamelCase<TRest>>}`
  : Lower<TValue>;

export type OneOf<TValue, TDefault> = TValue extends void ? TDefault : TValue;

// type Merge<L, R> = L extends void ? R
//   : R extends void ? L
//   : Omit<L, keyof R> & R;

export type Merge<TLeft, TRight> = TLeft extends void ? TRight
  : TRight extends void ? TLeft
  : TLeft & TRight;

export type MergeRecursive<TLeft, TRight> = TLeft extends void ? TRight
  : TRight extends void ? TLeft
  : TLeft & TRight;

export type ValueOf<TValue> = TValue extends Record<string, infer V>
  ? ValueOf<V>
  : TValue;
