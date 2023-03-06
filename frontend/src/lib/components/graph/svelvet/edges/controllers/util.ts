import type { StoreType } from '../../store/types/types';
import { writable, derived, get, readable } from 'svelte/store';
import type { AnchorType } from '../types/types';
import { getNodeById } from '../../nodes/controllers/util';

/**
 * Finds all Anchors that matches the conditions specified in the filter parameter from a Svelvet store and returns these Anchors in an array.
 *
 * @param store The Svelvet store containing the state of a Svelvet component
 * @param filter An object to specify conditions. Example: if filter = \{ sourceOrTarget: 'source', positionX: 35 \} then we will return all anchors with sourceOrTarget = source AND poxitionX = 35
 * @returns An array of Anchors that matches the conditions specified in the filter parameter
 */
export function getAnchors(store: StoreType, filter?: { [key: string]: any }) {
  let anchors = Object.values(get(store.anchorsStore));
  // filter the array of anchors for elements that match filter
  // Example: if filter = {sourceOrTarget: 'source', positionX: 35} then we will
  //return all anchors with sourceOrTarget = source AND poxitionX = 35
  if (filter !== undefined) {
    anchors = anchors.filter((anchor) => {
      for (let filterKey in filter) {
        const filterValue = filter[filterKey];
        if (anchor[filterKey as keyof AnchorType] !== filterValue) return false;
      }
      return true;
    });
  }
  // return list of anchors
  return anchors;
}

/**
 * Gets one anchor (source anchor or target anchor) from a given edge
 *
 * @param store The Svelvet store containing the state of the Svelvet component
 * @param edgeId The id of a given edge
 * @param sourceOrTarget A string of 'source' or 'target' to specify which anchor the function should return
 * @returns The source or target Anchor object of a given edge
 */
export function getAnchorFromEdge(
  store: StoreType,
  edgeId: string,
  sourceOrTarget: 'source' | 'target'
): AnchorType {
  const edge = getEdgeById(store, edgeId);
  const anchors = getAnchors(store, { edgeId: edgeId });
  if (anchors.length !== 2)
    throw `there should be two anchors for a given edge, there are ${anchors.length}`;
  // there should be one source anchor and one target anchor. Return the source anchor
  const anchor = anchors.filter(
    (anchor) => anchor.sourceOrTarget === sourceOrTarget
  );
  if (anchor.length !== 1)
    throw `there should only be one source/target anchor`;
  return anchor[0];
}

/**
 * getEdgeById will look for the targeted Edge that has the same id provided in the Svelvet component store.
 *
 * @param store The Svelvet store containing the state of a Svelvet component
 * @param id The id of the targeted Node
 * @returns The targeted Edge object in store.edgesStore
 */
export function getEdgeById(store: StoreType, id: string) {
  const edgesStore = get(store.edgesStore);
  const edge = edgesStore[id];
  if (edge === undefined) throw 'edge not found';
  return edge;
}

/**
 * getAnchorById will look for the targeted Anchor that has the same id in the Svelvet component store.
 *
 * @param store The Svelvet store containing the state of a Svelvet component
 * @param id The id of the targeted Anchor
 * @returns The target Anchor object in store.anchorsStore
 */
export function getAnchorById(store: StoreType, id: string) {
  const anchorsStore = get(store.anchorsStore);
  const anchor = anchorsStore[id];
  return anchor;
}
