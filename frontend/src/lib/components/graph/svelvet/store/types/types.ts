import type { CollapsibleType } from '../../collapsible/types/types';
import type { Writable } from 'svelte/store';
import type { AnchorType } from '../../edges/types/types';
import type { SvelteComponentTyped } from "svelte";

export interface ResizeNodeType {
  id: string;
  nodeId: string;
  edgeId?: string;
  canvasId: string;
  anchorId?: string;
  positionX: number;
  positionY: number;
  setPositionAndCascade: Function;
  setPosition: Function;
  delete: Function;
}

/*
Type for a single svelvet store
*/
export interface StoreType {
  nodesStore: Writable<{ [key: string]: NodeType }>;
  edgesStore: Writable<{ [key: string]: EdgeType }>;
  anchorsStore: Writable<{ [key: string]: AnchorType }>;
  resizeNodesStore: Writable<{ [key: string]: ResizeNodeType }>;
  potentialAnchorsStore: Writable<{ [key: string]: PotentialAnchorType }>;
  widthStore: Writable<number>;
  heightStore: Writable<number>;
  backgroundStore: Writable<boolean>;
  movementStore: Writable<boolean>;
  nodeIdSelected: Writable<number>;
  nodeSelected: Writable<boolean>; // this is used to stop d3 panning when node is being dragged
  d3Scale: Writable<number>; // for zoom and pan
  options: Writable<{ [key: string]: any }>;
  temporaryEdgeStore: Writable<TemporaryEdgeType[]>;
  nodeCreate: Writable<boolean>; // this option sets whether the "nodeEdit" feature is enabled
  boundary: Writable<boolean | PositionType>;
  edgeEditModal: Writable<null | string>; // this options is used to place the edgeEdit modal when an edge is right-clicked. null is no modal, positionType if modal should be placed at position defined by postionType.x, positionType.y
  collapsibleStore: Writable<CollapsibleType[]>;
  collapsibleOption: Writable<boolean>;
  lockedOption: Writable<boolean>;
  editableOption: Writable<boolean>;
  d3ZoomParameters: Writable<{
    [key: string]: number;
  }>;
  resizableOption: Writable<boolean>;
  highlightEdgesOption: Writable<boolean>;
}

export interface PositionType {
  x: number;
  y: number;
}

export interface NodeType {
  id: string;
  width: number;
  height: number;
  positionX: number;
  positionY: number;
  bgColor: string;
  data: { html?: any, custom?: { component: any, props?: any, cb?: (e: string, detail: any) => void }, img?: any, label?: string }
  canvasId: string;
  setPositionFromMovement: Function;
  delete: Function; //This is the method to delete the node from the store
  setSizeFromMovement: Function;
  setExportableData: Function;
  borderColor: string;
  image: boolean;
  src: string;
  textColor: string;
  borderRadius: number;
  childNodes: string[];
  className: string; //This is for custom className for node
  clickCallback: Function; // user-supplied callback that executes when the node is clicked
}

export interface EdgeType {
  id: string;
  sourceX: number;
  sourceY: number;
  targetX: number;
  targetY: number;
  canvasId: string;
  label: string;
  type: 'straight' | 'smoothstep' | 'step' | 'bezier';
  labelBgColor: string;
  labelTextColor: string;
  edgeColor: string;
  animate: boolean;
  noHandle: boolean;
  arrow: boolean;
  clickCallback: Function;
  className: string;
  delete: Function;
  setExportableData: Function;
}

export interface PotentialAnchorType {
  id: string;
  nodeId: string;
  callback: Function; // callback is used to calculate positionX, positionY based on parent node's data, and set the anchor position // TODO: rename to something better
  positionX: number;
  positionY: number;
  angle: number;
  canvasId: string;
  delete: Function;
}

export interface TemporaryEdgeType {
  id: string;
  sourcePotentialAnchorId: string; // this will always be set
  sourceX: number;
  sourceY: number;
  targetPotentialAnchorId: string | null; // this will be null until the temporary edge reaches another temporary anchor
  targetX: number;
  targetY: number;
  canvasId: string;
  type: string;
  edgeColor: string;
  createEdge: Function;
  createNode: Function;
}
