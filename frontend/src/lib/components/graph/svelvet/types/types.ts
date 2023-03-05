export interface UserNodeType {
  id: string;
  width: number;
  height: number;
  bgColor?: string;
  data: { html?: any, custom?: { component: any, props?: any, cb?: (e: string, detail: any) => void }, img?: any }
  position: { x: number; y: number };
  borderColor?: string | undefined;
  image?: boolean;
  src?: string;
  textColor?: string;
  targetPosition?: 'left' | 'right' | 'top' | 'bottom';
  sourcePosition?: 'left' | 'right' | 'top' | 'bottom';
  borderRadius?: number;
  childNodes?: string[];
  className?: string;
  clickCallback?: Function;
}

export interface UserEdgeType {
  id: string;
  source: string;
  target: string;
  sourceAnchorCb?: Function;
  targetAnchorCb?: Function;
  label?: string;
  labelBgColor?: string;
  labelTextColor?: string;
  edgeColor?: string;
  type?: 'straight' | 'smoothstep' | 'step' | 'bezier' | undefined;
  animate?: boolean;
  noHandle?: boolean;
  arrow?: boolean;
  clickCallback?: Function;
  className?: string;
}

import { findStore } from '../store/controllers/storeApi';
import { get } from 'svelte/store';
export function getD3PositionX(canvasId: string) {
  const store = findStore(canvasId);
  const width = get(store.widthStore);
  const x = width / 2 - get(store.d3ZoomParameters).x; // user input is shifted so that x=0, y=0 occurs in the center
  return x;
}
export function getD3PositionY(canvasId: string) {
  const store = findStore(canvasId);
  const height = get(store.heightStore);
  const y = height / 2 - get(store.d3ZoomParameters).y; // user input is shifted so that x=0, y=0 occurs in the center
  return y;
}
export function getD3Zoom(canvasId: string) {
  const store = findStore(canvasId);
  return get(store.d3ZoomParameters).k;
}
