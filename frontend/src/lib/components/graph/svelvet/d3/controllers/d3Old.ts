import { writable, derived, get, readable } from 'svelte/store';

export function d3ZoomCreator(
  nodeSelected,
  movementStore,
  backgroundStore,
  canvasId,
  gridSize,
  dotSize,
  d3Scale,
  d3
) {
  // TODO: Update d3Zoom type (refer to d3Zoom docs)
  let d3Zoom: any = d3
    .zoom()
    .filter(() => !get(nodeSelected))
    .scaleExtent([0.4, 2])
    .on('zoom', handleZoom);

  return d3Zoom;

  // function to handle zoom events - arguments: d3ZoomEvent
  function handleZoom(this: any, e: any): void {
    if (!get(movementStore)) return;

    //add a store that contains the current value of the d3-zoom's scale to be used in onMouseMove function
    d3Scale.set(e.transform.k);
    // should not run d3.select below if backgroundStore is false
    if (get(backgroundStore)) {
      d3.select(`#background-${canvasId}`)
        .attr('x', e.transform.x)
        .attr('y', e.transform.y)
        .attr('width', gridSize * e.transform.k)
        .attr('height', gridSize * e.transform.k)
        .selectAll('#dot')
        .attr('x', (gridSize * e.transform.k) / 2 - dotSize / 2)
        .attr('y', (gridSize * e.transform.k) / 2 - dotSize / 2)
        .attr('opacity', Math.min(e.transform.k, 1));
    }
    // transform 'g' SVG elements (edge, edge text, edge anchor)
    d3.select(`.Edges-${canvasId} g`).attr('transform', e.transform);
    // transform div elements (nodes)
    let transform = d3.zoomTransform(this);
    // selects and transforms all node divs from class 'Node' and performs transformation
    d3.select(`.Node-${canvasId}`)
      .style(
        'transform',
        'translate(' +
          transform.x +
          'px,' +
          transform.y +
          'px) scale(' +
          transform.k +
          ')'
      )
      .style('transform-origin', '0 0');
  }
}
