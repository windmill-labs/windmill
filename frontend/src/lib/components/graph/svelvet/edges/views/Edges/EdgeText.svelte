<script lang="ts">
  import type { EdgeTextProps } from './types';

  // destructuring props to pass into BaseEdge component
  export let edgeTextProps: EdgeTextProps;
  $: ({ label, labelBgColor, labelTextColor, centerX, centerY } =
    edgeTextProps);

  const shiftRectY: number = 7;
  $: pxRatio = label.length < 3 ? 9 : 7;

  // determine the center point of the edge to be used in the EdgeText component
  $: textCenterX = centerX;
  $: textCenterY = centerY;

  // determine width of rect to render based on label.length (removing spaces)
  // pxRatio is an estimate of how many pixels 1 character might take up
  // pxRatio not 100% accurate as font is not monospace
  $: spaces = label.split(' ').length - 1;
  $: newLength = label.length - spaces;
  $: labelPx = newLength * pxRatio;
</script>

{#if typeof label === 'undefined' || !label}
  {null}
{:else}
  <g>
    <rect
      class="EdgeTextBg"
      data-testid="edge-text-bg"
      fill={labelBgColor ? labelBgColor : 'white'}
      x={textCenterX - labelPx / 2}
      y={textCenterY - shiftRectY}
      width={labelPx}
      height={16}
    />
    <text
      class="EdgeText"
      x={textCenterX}
      y={textCenterY}
      font-size="12px"
      dominant-baseline="central"
      text-anchor="middle"
      fill={labelTextColor ? labelTextColor : 'black'}
    >
      {label}
    </text>
  </g>
{/if}
