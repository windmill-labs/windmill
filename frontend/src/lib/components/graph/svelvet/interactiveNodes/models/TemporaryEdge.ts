// table for the "connecting edge" feature
import type { TemporaryEdgeType } from '../../store/types/types';
import { getPotentialAnchorById } from '../controllers/util';
import { findStore } from '../../store/controllers/storeApi';
import { createEdgeAndAnchors } from '../controllers/createEdgeAndAnchors';

import { createNode } from '../controllers/createNode';

import { getNodeById } from '../..//nodes/controllers/util';

export class TemporaryEdge implements TemporaryEdgeType {
  constructor(
    public id: string,
    public sourcePotentialAnchorId: string, // this will always be set
    public sourceX: number,
    public sourceY: number,
    public targetPotentialAnchorId: string | null, // this will be null until the temporary edge reaches another temporary anchor
    public targetX: number,
    public targetY: number,
    public canvasId: string,
    public type: string,
    public edgeColor: string
  ) {}

  // HOW: (1) Get the source/target potentialAnchor object
  //      (3) Get node id's of corresponding source/target nodes
  //      (4)
  createEdge() {
    if (this.targetPotentialAnchorId === null)
      throw 'cannot create edge with null target';
    const store = findStore(this.canvasId);
    // get the source/target potentialAnchor objects
    const potentialAnchorSource = getPotentialAnchorById(
      store,
      this.sourcePotentialAnchorId
    );
    const potentialAnchorTarget = getPotentialAnchorById(
      store,
      this.targetPotentialAnchorId
    );
    // get node id's of source/target nodes
    const sourceNode = getNodeById(store, potentialAnchorSource.nodeId);
    const targetNode = getNodeById(store, potentialAnchorTarget.nodeId);
    createEdgeAndAnchors(store, sourceNode.id, targetNode.id, this.canvasId);
  }
  createNode() {
    if (this.targetPotentialAnchorId !== null)
      throw 'you should only create a node when you are hovering over blank space';
    const store = findStore(this.canvasId);
    // get the source/target potentialAnchor objects
    const potentialAnchorSource = getPotentialAnchorById(
      store,
      this.sourcePotentialAnchorId
    );
    const sourceNodeId = potentialAnchorSource.nodeId;
    createNode(store, sourceNodeId, this.targetX, this.targetY, this.canvasId);
  }
}
