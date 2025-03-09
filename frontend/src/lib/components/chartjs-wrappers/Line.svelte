<script lang="ts">
	import type { ChartData, ChartOptions } from 'chart.js'
	import type { HTMLCanvasAttributes } from 'svelte/elements'
	import Chart from './Chart.svelte'

  interface Props extends HTMLCanvasAttributes {
    data: ChartData<"line">;
    options: ChartOptions<"line">;
  }
  const props: Props = $props()
  
</script>

<Chart type="line" {...props} />