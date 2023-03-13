import { getRowsCount } from "./other";

export function getContainerHeight(items, yPerPx, cols) {
  return getRowsCount(items, cols) * yPerPx;
}
