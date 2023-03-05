// import { zoom, zoomTransform } from 'd3-zoom';
// import { select, selectAll } from 'd3-selection';
import { get } from 'svelte/store';

export function zoomInit(
  d3,
  canvasId,
  d3Zoom,
  d3Translate,
  initialLocation,
  initialZoom,
  d3Scale
) {
  //set default zoom logic
  d3.select(`.Edges-${canvasId}`)
    //makes sure translation is default at center coordinates
    .transition()
    .duration(0)
    .call(d3Zoom.translateTo, 0, 0)
    //moves camera to coordinates
    .transition()
    .duration(0)
    .call(
      d3Zoom.translateTo,
      initialLocation.x / initialZoom,
      initialLocation.y / initialZoom
    )
    // zooms in on selected point
    .transition()
    .duration(0)
    .call(d3Zoom.scaleTo, initialZoom.toFixed(2));
  // updates d3Translate with d3 object with x, y, and k values to be sent down to the minimap to be further calculated further
  d3Translate = d3.zoomIdentity
    .translate(initialLocation.x, initialLocation.y)
    .scale(initialZoom.toFixed(2));
  d3.select(`.Nodes-${canvasId}`)
    .transition()
    .duration(0)
    .call(d3Zoom.translateTo, 0, 0)
    .transition()
    .duration(0)
    .call(
      d3Zoom.translateTo,
      initialLocation.x / initialZoom,
      initialLocation.y / initialZoom
    )
    .transition()
    .duration(0)
    .call(d3Zoom.scaleTo, initialZoom.toFixed(2));
  // sets D3 scale to current k of object
  d3Scale.set(d3.zoomTransform(d3.select(`.Nodes-${canvasId}`)).k);
  return d3Translate;
}
// create d3 instance conditionally based on boundary prop
export function determineD3Instance(
  boundary,
  d3,
  nodeSelected,
  width,
  height,
  movementStore,
  backgroundStore,
  gridSize,
  dotSize,
  canvasId,
  d3Scale,
  handleZoom
) {
  if (boundary) {
    return d3
      .zoom()
      .filter(() => !get(nodeSelected))
      .scaleExtent([0.4, 4]) // limits for zooming in/out
      .translateExtent([
        [0, 0],
        [boundary.x, boundary.y],
      ]) // world extent
      .extent([
        [0, 0],
        [width, height],
      ])
      .on('zoom', handleZoom);
  } else {
    return d3
      .zoom()
      .filter(() => !get(nodeSelected))
      .scaleExtent([0.4, 2])
      .on('zoom', handleZoom);
  }
}
