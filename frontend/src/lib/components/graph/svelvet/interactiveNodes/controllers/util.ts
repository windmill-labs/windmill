import type { StoreType, PotentialAnchorType } from '../../store/types/types';
import { get } from 'svelte/store';
import type { NodeType } from '../../store/types/types';

/**
 * getPotentialAnchorById will look for the targeted potential Anchor that has the same id provided in the Svelvet component store.
 *
 * @param store The Svelvet store containing the state of a Svelvet component
 * @param id The id of the targeted potential Anchor
 * @returns The targeted potential Anchor object in store.potentialAnchorsStore
 */
export function getPotentialAnchorById(store: StoreType, id: string) {
  const potentialAnchorsStore = get(store.potentialAnchorsStore);
  const potentialAnchor = potentialAnchorsStore[id];
  if (potentialAnchor === undefined) throw 'potential anchor not found';
  return potentialAnchor;
}

/**
 * Finds all potentialAnchors that matches the conditions specified in the filter parameter from a Svelvet store and returns these potential anchors in an array
 *
 * @param store The Svelvet store containing the state of a Svelvet component
 * @param filter An object to specify conditions.
 * @returns An array of potential anchors that matches the conditions specified in filter parameter
 */
export function getPotentialAnchors(
  store: StoreType,
  filter?: { [key: string]: any }
) {
  let potentialAnchorsArr = Object.values(get(store.potentialAnchorsStore));
  // filter the array for elements that match filter
  if (filter !== undefined) {
    potentialAnchorsArr = potentialAnchorsArr.filter((potentialAnchor) => {
      for (let filterKey in filter) {
        const filterValue = filter[filterKey];
        if (
          potentialAnchor[filterKey as keyof PotentialAnchorType] !==
          filterValue
        )
          return false;
      }
      return true;
    });
  }
  // return list of anchors
  return potentialAnchorsArr;
}

/**
 * Finds all Nodes that matches the conditions specified in the filter parameter from a Svelvet store and returns these Nodes in an array
 *
 * @param store The Svelvet store containing the state of a Svelvet component
 * @param filter An object to specify conditions.
 * @returns An array of Node objects that matches the conditions specified in filter parameter. This array is non-reactive (ie, you cannot use information from this array to force a re-render of a Svelte component)
 */
export function getNodes(
  store: StoreType,
  filter?: { [key: string]: any }
): NodeType[] {
  let nodes = Object.values(get(store.nodesStore));
  // filter the array of anchors for elements that match filter
  // Example: if filter = {sourceOrTarget: 'source', positionX: 35} then we will
  //          return all anchors with sourceOrTarget = source AND poxitionX = 35
  if (filter !== undefined) {
    nodes = nodes.filter((node) => {
      for (let filterKey in filter) {
        const filterValue = filter[filterKey];
        if (node[filterKey as keyof NodeType] !== filterValue) return false;
      }
      return true;
    });
  }
  // return list of nodes
  return nodes;
}
