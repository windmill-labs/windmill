import type { StoreType } from '../../store/types/types';
import { get } from 'svelte/store';

/**
 * getNodeById will look for the targeted Node that has the same id provided in the Svelvet component store.
 *
 * @param store The Svelvet store containing the state of a Svelvet component
 * @param id The id of the targeted Node
 * @returns The targeted Node object in store.nodesStore
 */

export function getNodeById(store: StoreType, id: string) {
  const nodesStore = get(store.nodesStore);
  const node = nodesStore[id];
  return node;
}
