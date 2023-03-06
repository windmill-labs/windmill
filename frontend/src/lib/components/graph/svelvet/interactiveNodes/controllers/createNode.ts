const pkStringGenerator = () => (Math.random() + 1).toString(36).substring(7);
import type { StoreType, PotentialAnchorType } from '../../store/types/types';
import { get } from 'svelte/store';
import { Node } from '../../nodes/models/Node';
import { Edge } from '../../edges/models/Edge';
import { Anchor } from '../../edges/models/Anchor';

import {
  rightCb,
  leftCb,
  topCb,
  bottomCb,
} from '../../edges/controllers/anchorCbUser';
import { createPotentialAnchor } from '../../store/controllers/util';
import { dynamicCbCreator } from '../../edges/controllers/anchorCbDev';
import { getAnchors } from '../../edges/controllers/util';
import { getPotentialAnchors } from './util';

/**
 * Creates a new Node when user drags the Edge and releases mouse at a new spot. This functionality is needed for the "create new node by dragging" feature. See TemporaryEdge.createNode()
 * This function has no output, but alters the store. New Node/Edge/Anchor objects are added to the store.
 *
 * @param store The Svelvet store containing the state of a Svelvet component
 * @param sourceNodeId The id of the source Node
 * @param targetNodeX The postion X where user releases the mouse, which should be the x-axis position for the new Node
 * @param targetNodeY The postion Y where user releases the mouse, which should be the y-axis position for the new Node
 * @param canvasId The canvasId of a Svelvet component
 */
export function createNode(
  store: StoreType,
  sourceNodeId: string,
  targetNodeX: number,
  targetNodeY: number,
  canvasId: string
) {
  // create a new node
  const targetNodeId = pkStringGenerator();

  const node = new Node(
    targetNodeId,
    targetNodeX,
    targetNodeY,
    100, // width
    100, // height
    'white', // background color
    {
      label: 'new node',
    },
    canvasId,
    'black', // borderColor
    false, //  Node has an image
    '', // src, not sure what this does
    'black', // text color
    0, // borderRadius
    [],
    '',
    () => {}
  );

  // create new potential anchor, we do this because if we create
  const potentialAnchorRight = createPotentialAnchor(
    rightCb,
    store,
    targetNodeId,
    canvasId
  );
  const potentialAnchorLeft = createPotentialAnchor(
    leftCb,
    store,
    targetNodeId,
    canvasId
  );
  const potentialAnchorTop = createPotentialAnchor(
    topCb,
    store,
    targetNodeId,
    canvasId
  );
  const potentialAnchorBottom = createPotentialAnchor(
    bottomCb,
    store,
    targetNodeId,
    canvasId
  );

  // create an edge
  const edgeId = pkStringGenerator();
  const newEdge = new Edge(
    edgeId,
    -1,
    -1,
    -1,
    -1,
    canvasId,
    '',
    'bezier',
    'black',
    'black',
    'black',
    false,
    true,
    false,
    () => {},
    ''
  );

  // create source anchor
  const sourceAnchorId = pkStringGenerator();
  const sourceDynamicCb = dynamicCbCreator(store, edgeId, sourceAnchorId);
  const sourceAnchor = new Anchor(
    sourceAnchorId,
    sourceNodeId,
    edgeId,
    'source',
    -1, // dummy variables for x,y,angle for now
    -1, // dummy variables for x,y,angle for now
    sourceDynamicCb,
    canvasId,
    0 // dummy variables for x,y,angle for now
  );

  // create target anchor
  const targetAnchorId = pkStringGenerator();
  const targetDynamicCb = dynamicCbCreator(store, edgeId, targetAnchorId);
  const targetAnchor = new Anchor(
    targetAnchorId,
    targetNodeId,
    edgeId,
    'target',
    -1, // dummy variables for x,y,angle for now
    -1, // dummy variables for x,y,angle for now
    targetDynamicCb,
    canvasId,
    0 // dummy variables for x,y,angle for now
  );

  // put everything into the store
  const { nodesStore, edgesStore, anchorsStore, potentialAnchorsStore } = store;

  nodesStore.update((nodes) => {
    nodes[targetNodeId] = node;
    return { ...nodes };
  });

  anchorsStore.update((anchors) => {
    anchors[sourceAnchorId] = sourceAnchor;
    anchors[targetAnchorId] = targetAnchor;
    return { ...anchors };
  });
  potentialAnchorsStore.update((anchors) => {
    anchors[potentialAnchorRight.id] = potentialAnchorRight;
    anchors[potentialAnchorTop.id] = potentialAnchorTop;
    anchors[potentialAnchorBottom.id] = potentialAnchorBottom;
    anchors[potentialAnchorLeft.id] = potentialAnchorLeft;
    return { ...anchors };
  });
  edgesStore.update((edges) => {
    edges[edgeId] = newEdge;
    return { ...edges };
  });
  // make sure to update positions of anchors and potential anchors. TODO: don't need to do this for the entire store
  const anchors = getAnchors(store);
  for (const anchor of anchors) anchor.callback();
  const potAnchors = getPotentialAnchors(store);
  for (const anchor of potAnchors) anchor.callback();
}
