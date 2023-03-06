<script lang="ts">
  export let temporaryEdge;
  let path = '';
  $: {
    path = `M ${temporaryEdge.sourceX},${temporaryEdge.sourceY}L ${temporaryEdge.targetX},${temporaryEdge.targetY}`;
  }
</script>

<path
  class="animate"
  d={path}
  fill="transparent"
  stroke={'gray'}
  marker-end="url(#arrow)"
  aria-label="svg-path"
/>

<style>
  .animate {
    stroke-dasharray: 5;
    animation: dash 50000s linear;
  }
  @keyframes dash {
    from {
      stroke-dashoffset: 1000000;
    }
  }
</style>
