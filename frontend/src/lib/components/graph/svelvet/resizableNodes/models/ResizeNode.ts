import type { ResizeNodeType } from '../../store/types/types';
// import { writable, derived, get, readable } from 'svelte/store';
import { stores } from '../../store/models/store';

/** A ResizeNode class that implements ResizeNodeType interface
* @param {string} id The id of the ResizeNode
* @param {string} canvasId The canvasId of the Svelvet component that the instantiated ResizeNode will be on.
* @param {string} nodeId The id of the Node that the instantiated ResizedNode will be attached to.
* @param {number} positionX The X-axis position of the ResizeNode (left top corner of the ResizeNode)
* @param {number} positionY The Y-axis position of the ResizeNode (left top corner of the ResizeNode)
*/
export class ResizeNode implements ResizeNodeType {
  constructor(
    public id: string,
    public canvasId: string,
    public nodeId: string,
    public positionX: number,
    public positionY: number
  ) {}

  /**
   * setPosition will update the positionX and positionY of the ResizeNode. This will be invoked when the Node that this ResizeNode attached to is moving so that the ResizeNode will follow the Node.
   * 
   * @param movementX The mouse movement value on the X-axis 
   * @param movementY The mouse movement value on the Y-axis 
   */
  setPosition(movementX: number, movementY: number) {
    this.positionX += movementX;
    this.positionY += movementY;
  }

  /**
   * setPositionAndCascade will update the positionX and positionY of the ResizeNode and also cascade changes to related Node. This will be invoked when the user drags the ResizeNode so the position of the ResizeNode and the width and height of the Node would change accordingly.
   * @param movementX The mouse movement value on the X-axis
   * @param movementY The mouse movement value on the Y-axis
   */
  setPositionAndCascade(movementX: number, movementY: number) {
    //declare variables needed to interact with the corresponding node to this resize anchor
    const nodeId = this.nodeId;
    const { nodesStore } = stores[this.canvasId];

    //Updates the width/height of the corresponding Node
    nodesStore.update((nodes) => {
      const node = nodes[nodeId];

      //sets condition so node cannot be less than 20 px in width or height.
      if (node.width + movementX > 20 && node.height + movementY > 20) {
        //Updates position to this resizeNode anchor. Must be done within the update method so ensure the position doesn't change if the width goes below 20px
        this.positionX += movementX;
        this.positionY += movementY;
        //setSizeFromMovement will then update the anchors position as the width & height changes
        node.setSizeFromMovement(movementX, movementY);
      }
      return { ...nodes };
    });
  }
  
  /**
   * delete is going to delete the ResizeNode from the resizeNodesStore.
   */
  delete() {
    const store = stores[this.canvasId];
    const { resizeNodesStore } = stores[this.canvasId];
    resizeNodesStore.update((resizeNodes) => {
      for (const resizeNodeId in resizeNodes) {
        if (resizeNodes[resizeNodeId].id === this.id) {
          delete resizeNodes[resizeNodeId];
        }
      }
      return { ...resizeNodes };
    });
  }
}
