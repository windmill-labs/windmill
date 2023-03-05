<script lang="ts">
  import { findStore } from '../../store/controllers/storeApi';
  import { TemporaryEdge } from '../models/TemporaryEdge';
  import { writable, derived, get, readable } from 'svelte/store';
  import { getPotentialAnchorById } from '../controllers/util';
  import type {
    NodeType,
    EdgeType,
    StoreType,
    TemporaryEdgeType,
    PotentialAnchorType,
  } from '../../store/types/types';
  import type { UserNodeType, UserEdgeType } from '../../types/types';

  import { beforeUpdate, afterUpdate } from 'svelte';
  export let canvasId;
  export let x;
  export let y;
  export let potentialAnchorId: string;

  const store = findStore(canvasId);
  const potentialAnchor = getPotentialAnchorById(store, potentialAnchorId);

  let newEdge;
  let hovered = false;
  let anchorWidth = 8;
  let anchorHeight = 8;

  const { temporaryEdgeStore } = findStore(canvasId);
  let moving = false;
  let moved = false;
  let isDragging = false;

  $: mouseX = x;
  $: mouseY = y;
  // whether the mouse is hovering over the potential anchor
  // determines behavior of releasing the click; if the click is
  // hovering over an anchor then an edge is created then temporary edges
  // aer cleared, otherwise the temporary edges are cleared
  let mouseHover = false;
  const pkStringGenerator = () => (Math.random() + 1).toString(36).substring(7);
</script>

<svelte:window
  on:mousemove={(e) => {
    // imelements drawing an (temporary) edge from a potential anchor to the mouse cursor
    e.preventDefault();
    if (isDragging) {
      temporaryEdgeStore.update((edges) => {
        if (edges.length !== 1)
          throw `if you are dragging, there should be one edge. We have not implemented multiple edges`;
        const edge = edges[0];
        const d3Scale = get(store.d3Scale);
        edge.targetX += e.movementX / d3Scale;
        edge.targetY += e.movementY / d3Scale;
        return [edge];
      });
    }
  }}
  on:mouseup={(e) => {
    isDragging = false; // prevent the new edge from moving

    // only do update if temporaryEdgeStore has one element, and if that tempEdge.targetId = potentialAnchorId
    // otherwise don't do anything (don't even update at all!!!)
    if (
      get(temporaryEdgeStore).length === 1 &&
      get(temporaryEdgeStore)[0].targetPotentialAnchorId === potentialAnchorId
    ) {
      temporaryEdgeStore.update((edges) => {
        for (let edge of edges) {
          if (potentialAnchorId === edge.targetPotentialAnchorId) {
            edge.createEdge();
          }
        }
        return [];
      });
    } else if (
      get(temporaryEdgeStore).length === 1 &&
      get(temporaryEdgeStore)[0].targetPotentialAnchorId === null
    ) {
      temporaryEdgeStore.update((edges) => {
        for (let edge of edges) {
          // note length 1, refactor store to single tempEdge instead of arr of tempEdge
          edge.createNode();
        }
        return [];
      });
    }
  }}
/>

<div
  on:mouseenter={(e) => {
    // mouseHover is used to decide whether to create a new edge/node
    mouseHover = true;
    // our mouse is over anchor, so set the target anchor
    temporaryEdgeStore.update((edges) => {
      for (let edge of edges) {
        edge.targetPotentialAnchorId = potentialAnchorId;
      }
      return [...edges];
    });
  }}
  on:mouseleave={(e) => {
    // mouseHover is used to decide whether to create a new edge/node
    mouseHover = false;

    // If you are dragging an edge, and your mouse leaves an anchor area, then make sure to
    // set targetAnchor back to null. We do a check because there are multiple anchorpoints
    if (isDragging) {
      temporaryEdgeStore.update((edges) => {
        for (let edge of edges) {
          if (edge.targetPotentialAnchorId === potentialAnchorId)
            edge.targetPotentialAnchorId = null;
        }
        return [...edges];
      });
    }
  }}
  on:mousedown={(e) => {
    e.preventDefault();
    e.stopPropagation(); // Important! Prevents the event from firing on the parent element (the .Nodes div)
    isDragging = true;
    temporaryEdgeStore.update((edges) => {
      if (edges.length === 0) {
        // say there are no temporary edges. This means the user just clicked on the potential anchor
        // so the current mouse position = temporary anchor position = mouseX, mouseY
        const newTempEdge = new TemporaryEdge(
          pkStringGenerator(),
          potentialAnchor.id,
          mouseX,
          mouseY,
          null,
          mouseX,
          mouseY,
          canvasId,
          'straight',
          'black'
        );
        return [newTempEdge];
      } else {
        throw `there should only be zero or one temporary edge at any given time`;
      }
    });
  }}
  class="Anchor"
  style={`
      height:${anchorHeight}px;
      width:${anchorWidth}px;
      top: ${y - anchorHeight / 2}px;
      left:${x - anchorWidth / 2}px;
    `}
/>

<style>
  .Anchor {
    position: absolute;
    border-radius: 50%;
    cursor: crosshair;
    background-color: rgb(105, 99, 99);
    pointer-events: auto; /* this is needed for pointer events to work since we disable them in graphview */
  }

  .Anchor:hover {
    transform: scale(1.8) translateZ(-10px);
    background-color: #ff4742;
    box-shadow: 1px 1px 3px 1px rgba(0, 0, 0, 0.2);
  }
</style>
