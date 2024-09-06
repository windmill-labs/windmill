/** Alias method for Number constructor. */
// deno-lint-ignore no-explicit-any
export function parseNumber(value: any): number {
  return Number(value);
}

export type WidenType<T> = T extends string ? string
  : T extends number ? number
  : T extends boolean ? boolean
  : T;
