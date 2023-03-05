// this class is for the 4 potential anchors that appear on each node

import type { PotentialAnchorType } from '../../store/types/types';
import { stores } from '../../store/models/store';

/** Class representing an anchor with a Anchortype alias */
export class PotentialAnchor implements PotentialAnchorType {
  /**
   * creates a potential anchor.
   * Users can create edges by linking two potential anchors
   */
  constructor(
    public id: string,
    public nodeId: string,
    public callback: Function, // callback is used to calculate positionX, positionY based on parent node's data, and set the anchor position // TODO: rename to something better
    public positionX: number,
    public positionY: number,
    public angle: number,
    public canvasId: string
  ) {}

  delete() {
    const { potentialAnchorsStore } = stores[this.canvasId];

    potentialAnchorsStore.update((potentialAnchors) => {
      for (const potentialAnchorId in potentialAnchors) {
        if (potentialAnchors[potentialAnchorId].id === this.id) {
          delete potentialAnchors[potentialAnchorId];
        }
      }
      return { ...potentialAnchors };
    });
  }
}
