<script>
  import BaseEdge from './BaseEdge.svelte';
  import { getCenter } from './utils';

  const Position = {};
  Position['Left'] = 'left';
  Position['Right'] = 'right';
  Position['Top'] = 'top';
  Position['Bottom'] = 'bottom';
  // These are some helper methods for drawing the round corners
  // The name indicates the direction of the path. "bottomLeftCorner" goes
  // from bottom to the left and "leftBottomCorner" goes from left to the bottom.
  // We have to consider the direction of the paths because of the animated lines.
  const bottomLeftCorner = (x, y, size) =>
    `L ${x},${y - size}Q ${x},${y} ${x + size},${y}`;
  const leftBottomCorner = (x, y, size) =>
    `L ${x + size},${y}Q ${x},${y} ${x},${y - size}`;
  const bottomRightCorner = (x, y, size) =>
    `L ${x},${y - size}Q ${x},${y} ${x - size},${y}`;
  const rightBottomCorner = (x, y, size) =>
    `L ${x - size},${y}Q ${x},${y} ${x},${y - size}`;
  const leftTopCorner = (x, y, size) =>
    `L ${x + size},${y}Q ${x},${y} ${x},${y + size}`;
  const topLeftCorner = (x, y, size) =>
    `L ${x},${y + size}Q ${x},${y} ${x + size},${y}`;
  const topRightCorner = (x, y, size) =>
    `L ${x},${y + size}Q ${x},${y} ${x - size},${y}`;
  const rightTopCorner = (x, y, size) =>
    `L ${x - size},${y}Q ${x},${y} ${x},${y + size}`;
  // returns string to pass into edge 'path' svg d attribute (where to be drawn)
  export function getSmoothStepPath({
    sourceX,
    sourceY,
    sourcePosition = Position.Bottom,
    targetX,
    targetY,
    targetPosition = Position.Top,
    borderRadius = 5,
    centerX,
    centerY,
  }) {
    const [_centerX, _centerY, offsetX, offsetY] = getCenter({
      sourceX,
      sourceY,
      targetX,
      targetY,
    });
    const cornerWidth = Math.min(borderRadius, Math.abs(targetX - sourceX));
    const cornerHeight = Math.min(borderRadius, Math.abs(targetY - sourceY));
    const cornerSize = Math.min(cornerWidth, cornerHeight, offsetX, offsetY);
    const leftAndRight = [Position.Left, Position.Right];
    const cX = typeof centerX !== 'undefined' ? centerX : _centerX;
    const cY = typeof centerY !== 'undefined' ? centerY : _centerY;
    let firstCornerPath = null;
    let secondCornerPath = null;
    // for non-mixed edge top/bottom
    if (sourceX <= targetX) {
      firstCornerPath =
        sourceY <= targetY
          ? bottomLeftCorner(sourceX, cY, cornerSize)
          : topLeftCorner(sourceX, cY, cornerSize);
      secondCornerPath =
        sourceY <= targetY
          ? rightTopCorner(targetX, cY, cornerSize)
          : rightBottomCorner(targetX, cY, cornerSize);
    } else {
      firstCornerPath =
        sourceY < targetY
          ? bottomRightCorner(sourceX, cY, cornerSize)
          : topRightCorner(sourceX, cY, cornerSize);
      secondCornerPath =
        sourceY < targetY
          ? leftTopCorner(targetX, cY, cornerSize)
          : leftBottomCorner(targetX, cY, cornerSize);
    }
    // for non-mixed edge left/right
    if (
      leftAndRight.includes(sourcePosition) &&
      leftAndRight.includes(targetPosition)
    ) {
      if (sourceX <= targetX) {
        firstCornerPath =
          sourceY <= targetY
            ? rightTopCorner(cX, sourceY, cornerSize)
            : rightBottomCorner(cX, sourceY, cornerSize);
        secondCornerPath =
          sourceY <= targetY
            ? bottomLeftCorner(cX, targetY, cornerSize)
            : topLeftCorner(cX, targetY, cornerSize);
      } else if (
        (sourcePosition === Position.Right &&
          targetPosition === Position.Left) ||
        (sourcePosition === Position.Left &&
          targetPosition === Position.Right) ||
        (sourcePosition === Position.Left && targetPosition === Position.Left)
      ) {
        // and sourceX > targetX
        firstCornerPath =
          sourceY <= targetY
            ? leftTopCorner(cX, sourceY, cornerSize)
            : leftBottomCorner(cX, sourceY, cornerSize);
        secondCornerPath =
          sourceY <= targetY
            ? bottomRightCorner(cX, targetY, cornerSize)
            : topRightCorner(cX, targetY, cornerSize);
      }
      // for mixed edges (top/bottom to left/right) OR (left/right to top/bottom)
    } else if (
      leftAndRight.includes(sourcePosition) &&
      !leftAndRight.includes(targetPosition)
    ) {
      if (sourceX <= targetX) {
        firstCornerPath =
          sourceY <= targetY
            ? rightTopCorner(targetX, sourceY, cornerSize)
            : rightBottomCorner(targetX, sourceY, cornerSize);
      } else {
        firstCornerPath =
          sourceY <= targetY
            ? leftTopCorner(targetX, sourceY, cornerSize)
            : leftBottomCorner(targetX, sourceY, cornerSize);
      }
      secondCornerPath = '';
    } else if (
      !leftAndRight.includes(sourcePosition) &&
      leftAndRight.includes(targetPosition)
    ) {
      if (sourceX <= targetX) {
        firstCornerPath =
          sourceY <= targetY
            ? bottomLeftCorner(sourceX, targetY, cornerSize)
            : topLeftCorner(sourceX, targetY, cornerSize);
      } else {
        firstCornerPath =
          sourceY <= targetY
            ? bottomRightCorner(sourceX, targetY, cornerSize)
            : topRightCorner(sourceX, targetY, cornerSize);
      }
      secondCornerPath = '';
    }
    return `M ${sourceX},${sourceY}${firstCornerPath}${secondCornerPath}L ${targetX},${targetY}`;
  }
  export let edge;
  export let borderRadius = 5;
  export let canvasId;

  import { findStore } from '../../../store/controllers/storeApi';
  import { getAnchorFromEdge } from '../../../edges/controllers/util';

  let params;
  $: {
    const store = findStore(canvasId);
    const sourceAnchor = getAnchorFromEdge(store, edge.id, 'source');
    const targetAnchor = getAnchorFromEdge(store, edge.id, 'target');
    const mapAngle = { 0: 'right', 90: 'top', 180: 'left', 270: 'bottom' };
    params = {
      sourceX: edge.sourceX,
      sourceY: edge.sourceY,
      targetX: edge.targetX,
      targetY: edge.targetY,
      sourcePosition: mapAngle[sourceAnchor.angle],
      targetPosition: mapAngle[targetAnchor.angle],
      borderRadius: borderRadius,
    };
  }
  $: [centerX, centerY] = getCenter(params);
  $: path = getSmoothStepPath(params);
  $: baseEdgeProps = {
    ...edge,
    path: path,
    centerX: centerX,
    centerY: centerY,
  };
</script>

<BaseEdge {baseEdgeProps} {canvasId} />
