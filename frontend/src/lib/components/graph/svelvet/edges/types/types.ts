export type AnchorCbType = {
  (): void;
  type: 'dynamic' | 'fixed';
};

export interface AnchorType {
  id: string; // note that the user never specifies an anchor and they are generated dynamically. id will be a random string.
  nodeId: string;
  edgeId: string;
  sourceOrTarget: 'source' | 'target';
  positionX: number;
  positionY: number;
  callback: AnchorCbType; // callback is used to calculate positionX, positionY based on parent node's data, and set the anchor position // TODO: rename to something better
  angle: number;
  setPositionFromNode: Function;
  setPosition: Function;
  updateEdges: Function;
  getOtherAnchorId: Function;
}
