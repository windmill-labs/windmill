import type { StoreType, ResizeNodeType } from '../../store/types/types';
import { get } from 'svelte/store';
/**
 * Finds all resizeNodes that matches the conditions specified in the filter parameter from a Svelvet store and returns these resizeNodes in an array
 *
 * @param store The Svelvet store containing the state of a Svelvet component
 * @param filter An object to specify conditions.
 * @returns An array of resizeNode objects that matches the conditions specified in filter parameter
 */
export function getResizeNodes(
  store: StoreType,
  filter?: { [key: string]: any }
) {
  let resizeNodes = Object.values(get(store.resizeNodesStore));
  // filter the array for elements that match filter
  if (filter !== undefined) {
    resizeNodes = resizeNodes.filter((resizeNode) => {
      for (let filterKey in filter) {
        const filterValue = filter[filterKey];
        if (resizeNode[filterKey as keyof ResizeNodeType] !== filterValue)
          return false;
      }
      return true;
    });
  }
  // return list of anchors
  return resizeNodes;
}
