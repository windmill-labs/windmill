/*
This file contains "middleware" functions that sanitize user input (UserNodeType and UserEdgeType). Put functions that 
maintain consistency between previous 

*/
import { get } from 'svelte/store';
import {
  bottomCb,
  leftCb,
  rightCb,
  topCb,
} from '../../edges/controllers/anchorCbUser';
import type { StoreType } from '../../store/types/types';
import type { UserEdgeType, UserNodeType } from '../../types/types';

/**
 * sanitizeCanvasOptions will sanitize the canvas level options so that incompatible features will not be run simulataneously
 * @param store The array of nodes that have a UserNodeType
 * @returns void. The store is modified directly
 */
export function sanitizeCanvasOptions(store: StoreType) {
  enforceCollapsibleCompatibility(store);
}

function enforceCollapsibleCompatibility(store: StoreType) {
  if (get(store.collapsibleOption)) {
    store.nodeCreate.set(false);
  }
}

/**
 * sanitizeUserNodesAndEdges will sanitize the data initially passed in to Svelvet component. For example, the node that user specified have an integar as its id but to instantiate a Node and be compatible with uuid we will need to convert the integar id to a string.
 * @param userNodes The array of nodes that have a UserNodeType
 * @param userEdges The array of edges that have a UserEdgeType
 * @returns An object of sanitized userNodes and userEdges
 */
export function sanitizeUserNodesAndEdges(
  userNodes: UserNodeType[],
  userEdges: UserEdgeType[]
) {
  convertIdToString(userNodes);
  convertEdgeIdsToString(userEdges);
  convertAnchorPositionsToCallbacks(userNodes, userEdges);
  setDefaultEdgeType(userEdges);
  return { userNodes, userEdges };
}

/**
 * setDefaultEdgeType ensures a default edge type of 'bezier' if the user does not set one, or sets a nonsense type.
 * @param userEdges The array of edges that have a UserEdgeType
 * @returns No return, the function edits the objects in place
 */
function setDefaultEdgeType(userEdges) {
  for (let userEdge of userEdges) {
    if (!['smoothstep', 'step', 'bezier', 'straight'].includes(userEdge.type))
      userEdge.type = 'bezier';
  }
}

/**
 * convertAnchorPositionsToCallbacks
 * @description
 * WHY: This function is in order to maintain compliance with earlier versions of Svelvet.
 * HISTORY: In Svelvet<=5, anchor points were hard-coded onto each node. Each node had a "sourcePosition"
 * "targetPosition" where the edges would be attached. In Svelvet6, the store was re-designed
 * from the ground up into an object-relational data model where anchor points could be attached
 * at any point on the node using callbacks. This enabled features such as custom edge position,
 * adaptive edge positioning, and dynamic edges.
 * The purpose of this function is to parse the old way of specifying edge positions (as two source/target
 * anchors on the node) into the Svelvet6 (where anchors are separate objects) in order to maintain a
 * consistent user experience. However, we suggest that this functionality (parsing Svelvet5 syntax into
 * Svelvet6 syntax) be removed completely in favor of only using Svelvet6 syntax in order to reduce edge cases.
 */
function convertAnchorPositionsToCallbacks(
  userNodes: UserNodeType[],
  userEdges: UserEdgeType[]
) {
  // convert userNodes array into object for constant time lookup by id
  const userNodesObj: { [key: string]: UserNodeType } = {};
  for (let userNode of userNodes) userNodesObj[userNode.id] = userNode;

  // iterate through userEdges, and check the source/target nodes.
  for (let userEdge of userEdges) {
    const userNodeSource = userNodesObj[userEdge.source];
    const userNodeTarget = userNodesObj[userEdge.target];
    const sourcePosition = userNodeSource.sourcePosition;
    const targetPosition = userNodeTarget.targetPosition;

    const cbs = { left: leftCb, right: rightCb, top: topCb, bottom: bottomCb };
    if (sourcePosition) userEdge.sourceAnchorCb = cbs[sourcePosition];
    if (targetPosition) userEdge.targetAnchorCb = cbs[targetPosition];
  }
}

/**
 * Converts node id's to strings. For Svelvet<6, node id's were numbers. These were switched to strings for compatibility with uuid. This function does not return a new array, instead it mutates and sanitizes the original array.
 * @param userNodes The array of userNodes (not yet sanitized)
 */
function convertIdToString(userNodes: UserNodeType[]) {
  userNodes = userNodes.map((node) => {
    node.id = node.id.toString();
    node.childNodes =
      node.childNodes === undefined
        ? []
        : node.childNodes.map((childId) => childId.toString());
    return node;
  });
}

/**
 * Converts source/target node id's to string. For Svelvet<6, id's were numbers. These were switched to strings for compatibility with uuid. This function does not return a new array, instead it mutates and sanitizes the original array.
 * @param userEdges The array of userEdges (not yet sanitized)
 */
function convertEdgeIdsToString(userEdges: UserEdgeType[]) {
  userEdges = userEdges.map((edge) => {
    edge.source = edge.source.toString();
    edge.target = edge.target.toString();
    return edge;
  });
}
