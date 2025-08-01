export type Order = Array<string | symbol>;
export type Item = {
  [k in PropertyKey]: unknown | Item;
}

export const ORDER_KEY_ID: string;
export default function<I extends Item = Item>(obj: I, order?: Order): I;
export function getOrder<I extends Item = Item>(obj: I): Order | undefined;
export function setOrder<I extends Item = Item>(obj: I, order: Order): boolean;
export function deserialize<I extends Item = Item>(obj: I, deep: boolean): I;
export function serialize<I extends Item = Item>(obj: I, deep: boolean): I;
export function isOrderedObject<I extends Item = Item>(obj: I): boolean;


