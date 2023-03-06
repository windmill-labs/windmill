import { findStore } from './storeApi';
import { get } from 'svelte/store';
export function getD3PositionX(canvasId) {
  const store = findStore(canvasId);
  const width = get(store.widthStore);
  const x = width / 2 - get(store.d3ZoomParameters).x; // user input is shifted so that x=0, y=0 occurs in the center
  return x;
}
export function getD3PositionY(canvasId) {
  const store = findStore(canvasId);
  const height = get(store.heightStore);
  const y = height / 2 - get(store.d3ZoomParameters).y; // user input is shifted so that x=0, y=0 occurs in the center
  return y;
}
export function getD3Zoom(canvasId) {
  const store = findStore(canvasId);
  return get(store.d3ZoomParameters).k;
}
