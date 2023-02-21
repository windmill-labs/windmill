import { makeMatrixFromItems } from "../utils/matrix.js";
import { findFreeSpaceForItem, normalize, adjust } from "../utils/item.js";
import { getRowsCount } from "./other.js";

function makeItem(item) {
  const { min = { w: 1, h: 1 }, max } = item;
  return {
    fixed: false,
    resizable: !item.fixed,
    draggable: !item.fixed,
    customDragger: false,
    customResizer: false,
    min: {
      w: Math.max(1, min.w),
      h: Math.max(1, min.h),
    },
    max: { ...max },
    ...item,
  };
}

const gridHelp = {
  normalize(items, col) {
    const rows = getRowsCount(items, col);
    return normalize(items, col, rows);
  },

  adjust(items, col) {
    return adjust(items, col);
  },

  item(obj) {
    return makeItem(obj);
  },

  findSpace(item, items, cols) {
    let matrix = makeMatrixFromItems(items, getRowsCount(items, cols), cols);

    let position = findFreeSpaceForItem(matrix, item[cols]);
    return position;
  },
};

export default gridHelp;
