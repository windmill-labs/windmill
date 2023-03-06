<script lang="ts">
  import { findStore } from '../../store/controllers/storeApi';
  import type { ResizeNodeType } from '../../store/types/types';
  import { writable, derived, get, readable } from 'svelte/store';

  export let resizeId: string;
  export let canvasId: string;

  const store = findStore(canvasId);
  const { resizeNodesStore, nodeSelected } = store;

  let isSelected = false;
  let reactResizeNode: ResizeNodeType;
  $: reactResizeNode = $resizeNodesStore[resizeId];
</script>

<svelte:window
  on:mousemove={(e) => {
    e.preventDefault();
    if (isSelected) {
      resizeNodesStore.update((resNode) => {
        const newResNode = resNode[resizeId];
        const d3Scale = get(store.d3Scale);
        newResNode.setPositionAndCascade(
          e.movementX / d3Scale,
          e.movementY / d3Scale,
          resizeId
        );
        return { ...resNode };
      });
    }
  }}
  on:mouseup={(e) => {
    e.preventDefault();
    // don't need to set $nodeSelected = false because Node component will do it for you
    isSelected = false;
  }}
/>

<div
  on:mousedown={(e) => {
    e.preventDefault();
    $nodeSelected = true;
    isSelected = true;
  }}
  on:mouseover={(e) => ($nodeSelected = true)}
  on:focus
  on:mouseleave={(e) => ($nodeSelected = false)}
  on:mouseenter={(e) => ($nodeSelected = true)}
  on:wheel|preventDefault
  class="ResizeNode"
  style="
  left: {reactResizeNode.positionX - 10}px;
  top: {reactResizeNode.positionY - 10}px;
  width: {20}px;
  height: {20}px;
  background-color: purple;
  border-color: purple;
  border-radius: 50%;"
  id="svelvet-{resizeId}"
/>

<style>
  .ResizeNode {
    position: absolute;
    display: grid;
    user-select: none;
    cursor: move;
    justify-content: center;
    overscroll-behavior: auto;
    align-items: center;
    font-size: 14px;
    text-align: center;
    border: solid 1px black;
    border-radius: 5px;
    box-shadow: 1px 1px 3px 1px rgba(0, 0, 0, 0.2);
    opacity: 0;
    pointer-events: auto; /* this is needed for pointer events to work since we disable them in graphview */
  }

  .ResizeNode:hover {
    cursor: nwse-resize;
    opacity: 0.5;
  }
</style>
