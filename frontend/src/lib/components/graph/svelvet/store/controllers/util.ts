const pkStringGenerator = () => (Math.random() + 1).toString(36).substring(7);
import {
  rightCb,
  leftCb,
  topCb,
  bottomCb,
} from '../../edges/controllers/anchorCbUser'; // these are callbacks used to calculate anchor position relative to node
import {
  dynamicCbCreator,
  fixedCbCreator,
} from '../../edges/controllers/anchorCbDev';

import type { AnchorType, AnchorCbType } from '../../edges/types/types';

import type {
  NodeType,
  EdgeType,
  StoreType,
  ResizeNodeType,
  PotentialAnchorType,
} from '../types/types';
import type { UserNodeType, UserEdgeType } from '../../types/types';

import { ResizeNode } from '../../resizableNodes/models/ResizeNode';
import { Anchor } from '../../edges/models/Anchor';
import { Node } from '../../nodes/models/Node';
import { Edge } from '../../edges/models/Edge';
import { writable, derived, get, readable } from 'svelte/store';
import { getEdgeById, getAnchors } from '../../edges/controllers/util';

/**
 * Creates resize node on the bottom right corner of the targeted Node
 * @param canvasId The canvasId of the Svelvet component that holds the targeted Node
 * @param nodeId The id of the Node that the resize node attached to
 * @param posX The number of pixels on the x-axis relative to the left top corner of the targeted Node
 * @param posY The number of pixels on the y-axis relative to the left top corner of the targeted Node
 * @returns A ReziseNode object with randomized id, canvasId, nodeId, posX, and posY
 */
function createResizeNode(
  canvasId: string,
  nodeId: string,
  posX: number,
  posY: number
) {
  const id = pkStringGenerator();
  const resizeNode = new ResizeNode(id, canvasId, nodeId, posX, posY);
  return resizeNode;
}

/**
 * Creates an Anchor on the targeted Node with infomation the userNode holds
 * @param store An object containing the state of the Svelvet component. You can access the following through `store`: nodesStore, edgesStore, anchorsStore, etc.
 * @param userNode A node that the user specifies. This is NOT the same as a Node object.
 * @param sourceOrTarget User specified information of source or target
 * @param canvasId The id of the canvas that holds the Anchor and its attached Node
 * @param edge An edge that the user specifies. It should target the userNode as source or target. This is NOT the same as an Edge object
 * @returns An Anchor object
 */

function createAnchor(
  store: StoreType,
  userNode: UserNodeType | null,
  sourceOrTarget: 'source' | 'target',
  canvasId: string,
  edge: UserEdgeType
) {
  // edge case
  if (userNode === null)
    throw `you cannot create an anchor without a user node (for now)`;

  const edgeId = edge.id;
  const anchorId = pkStringGenerator();

  // userCb is the appropriate source or taret callback from userEdge object. It is
  // possible the user to NOT set userCb in which case userCb will be undefined
  let userCb: Function | undefined;
  if (sourceOrTarget === 'target') userCb = edge.targetAnchorCb;
  else userCb = edge.sourceAnchorCb;

  // create anchor callbacks

  let cb: AnchorCbType;
  if (userCb === undefined) cb = dynamicCbCreator(store, edgeId, anchorId);
  else cb = fixedCbCreator(store, edgeId, anchorId, userNode.id, userCb);

  // Create a new anchor.
  const anchor = new Anchor(
    anchorId,
    userNode.id,
    edgeId,
    sourceOrTarget,
    -1, // dummy variables for x,y,angle for now
    -1, // dummy variables for x,y,angle for now
    cb,
    canvasId,
    0 // dummy variables for x,y,angle for now
  );
  // return
  return anchor;
}

/**
 * Populates edgesStore of Edges. This function does not return the edgesStore. Instead it sets the nodesStore of Svelvet store.
 * @param store An object containing the state of the Svelvet component. You can access the following through `store`: nodesStore, edgesStore, anchorsStore, etc.
 * @param edges An edge that the user specifies. This is NOT the same as a Edge object.
 * @param canvasId The canvasId of the Svelvet component that holds the Edges
 */
export function populateEdgesStore(
  store: StoreType,
  edges: UserEdgeType[],
  canvasId: string
) {
  const edgesStore: { [key: string]: EdgeType } = {};
  for (let i = 0; i < edges.length; i++) {
    const userEdge = edges[i];
    //  { id: 'e1-2', source: 1, type: 'straight', target: 2, label: 'e1-2' },
    // source is node.id for the source node
    // target is node.id for the target node
    // We need to get the anchors
    const {
      source: sourceNodeId,
      target: targetNodeId,
      id: edgeId,
      type,
      label,
      labelBgColor,
      labelTextColor,
      edgeColor,
      animate,
      noHandle,
      arrow,
      clickCallback,
      className,
    } = userEdge;

    const anchors = getAnchors(store, { edgeId: edgeId });
    // check that we have two anchors for every edge
    if (anchors.length !== 2) throw 'We should have two anchors for every node';
    // check that we have 1 source anchor and 1 target anchor. Since sourceOrTarget is typed to be either 'source'
    //   or 'target', it suffices to check whether there are two unique elements
    if (new Set(anchors.map((e) => e.sourceOrTarget)).size !== 2)
      throw 'we should have one source and one target anchor';
    // get source and target anchor
    let sourceAnchor, targetAnchor;
    if (anchors[0].sourceOrTarget === 'source') {
      sourceAnchor = anchors[0];
      targetAnchor = anchors[1];
    } else {
      sourceAnchor = anchors[1];
      targetAnchor = anchors[0];
    }

    edgesStore[edgeId] = new Edge(
      edgeId,
      sourceAnchor.positionX,
      sourceAnchor.positionY,
      targetAnchor.positionX,
      targetAnchor.positionY,
      canvasId,
      userEdge.label === undefined ? '' : userEdge.label,
      userEdge.type === undefined ? 'bezier' : userEdge.type,
      userEdge.labelBgColor === undefined ? 'white' : userEdge.labelBgColor,
      userEdge.labelTextColor === undefined ? 'black' : userEdge.labelTextColor,
      userEdge.edgeColor === undefined ? 'black' : userEdge.edgeColor,
      userEdge.animate === undefined ? false : userEdge.animate,
      userEdge.noHandle === undefined ? false : userEdge.noHandle,
      userEdge.arrow === undefined ? false : userEdge.arrow,
      userEdge.clickCallback === undefined ? () => { } : userEdge.clickCallback,
      userEdge.className === undefined ? '' : userEdge.className
    );
  }
  store.edgesStore.set(edgesStore);
}

/**
 * Finds userNode (with UserNodeType; Not the same as the Node) by the node id from nodesStore
 * @param id The id of the Node in its nodesStore
 * @param userNodes The array of userNodes (NOT the same as Node object)
 * @returns The node that user specified or null if not found
 */
function findUserNodeById(
  id: string,
  userNodes: UserNodeType[]
): UserNodeType | null {
  for (let i = 0; i < userNodes.length; i++) {
    const userNode = userNodes[i];
    if (userNode.id === id) return userNode;
  }
  return null;
}


/**
 * Populates the anchorsStore. This will overwrite any data in the AnchorsStore.
 * @param store The Svelvet store containing the state of the Svelvet component
 * @param nodes An array of user specified nodes
 * @param edges An array of user specified edges
 * @param canvasId The canvasId of the Svelvet component that holds the nodes and edges
 */
export function populateAnchorsStore(
  store: StoreType,
  nodes: UserNodeType[],
  edges: UserEdgeType[],
  canvasId: string
) {
  // anchorsStore will populated and eventaully synchronized to store.anchorsStore
  const anchorsStore: { [key: string]: AnchorType } = {};
  // iterate through user edges. Note the user never explicitly defines anchors; we calculate anchors
  // from the user edge/node information
  for (let i = 0; i < edges.length; i++) {
    const userEdge = edges[i];
    // find the source and target userNodes. These will be used to create the nodeId foreign key and
    // determine placement of the anchor based on userNode.targetPosition, useNode.sourcePosition
    const { source: sourceNodeId, target: targetNodeId } = userEdge;
    const sourceUserNode = findUserNodeById(sourceNodeId, nodes);
    const targetUserNode = findUserNodeById(targetNodeId, nodes);
    // create source anchor
    const sourceAnchor = createAnchor(
      store,
      sourceUserNode,
      'source',
      canvasId,
      userEdge
    );
    // create target anchor
    const targetAnchor = createAnchor(
      store,
      targetUserNode,
      'target',
      canvasId,
      userEdge
    );
    // store source and target anchors
    anchorsStore[sourceAnchor.id] = sourceAnchor;
    anchorsStore[targetAnchor.id] = targetAnchor;
  }

  //populates the anchorsStore
  store.anchorsStore.set(anchorsStore);

  // set anchor positions. We can only set anchor positions after anchorsStore and nodesStore
  // has been populated. TODO: maybe add a check to see that anchorsStore and NodesStore populated?
  const anchors = getAnchors(store);
  for (const anchor of anchors) anchor.callback();
}

/**
 * Populates the nodesStore. This will overwrite any data in the nodesStore.
 * @param store The Svelvet store containing the state of the Svelvet component
 * @param nodes An array of user specifed nodes
 * @param canvasId The canvasId of the Svelvet component that holds the nodes
 */
export function populateNodesStore(
  store: StoreType,
  nodes: UserNodeType[],
  canvasId: string
) {
  // this is the nodesStore object. THIS IS NOT THE SAME AS A NODESTORE
  const nodesStore: { [key: string]: NodeType } = {};
  // iterate through user nodes and create node objects
  for (let i = 0; i < nodes.length; i++) {
    const userNode: UserNodeType = nodes[i];
    const nodeId = userNode.id;

    // TODO: move sanitizing default values to middleware
    const node = new Node(
      nodeId.toString(),
      userNode.position.x,
      userNode.position.y,
      userNode.width,
      userNode.height,
      userNode.bgColor ?? 'white',
      userNode.data,
      canvasId,
      userNode.borderColor === undefined ? 'black' : userNode.borderColor,
      userNode.image === undefined ? false : userNode.image,
      userNode.src === undefined ? '' : userNode.src,
      userNode.textColor === undefined ? '' : userNode.textColor,
      userNode.borderRadius === undefined ? 0 : userNode.borderRadius,
      userNode.childNodes === undefined ? [] : userNode.childNodes,
      userNode.className === undefined ? '' : userNode.className,
      userNode.clickCallback === undefined ? () => { } : userNode.clickCallback
    );

    nodesStore[nodeId] = node;
  }
  // This is actually what sets the store
  store.nodesStore.set(nodesStore);
}

/**
 * Populates the resizeNodeStore. If a Node is resizable, a small ResizeNode object is going to be attached to the Node's right bottom corner to react to the mouse drag.
 * @param store The Svelvet store containing the state of the Svelvet component
 * @param nodes An array of user specifed nodes (NOT the same as Node)
 * @param canvasId The canvasId of the Svelvet component that holds the resizeNodes
 */
export function populateResizeNodeStore(
  store: StoreType,
  nodes: UserNodeType[],
  canvasId: string
) {
  const resizeNodeStore: { [key: string]: ResizeNodeType } = {};

  for (let i = 0; i < nodes.length; i++) {
    const userNode = nodes[i];
    const { id, width, height, position } = userNode;
    const resizeNode = createResizeNode(
      canvasId,
      id,
      position.x + width,
      position.y + height
    );
    resizeNodeStore[resizeNode.id] = resizeNode;
  }

  store.resizeNodesStore.set(resizeNodeStore);
}
