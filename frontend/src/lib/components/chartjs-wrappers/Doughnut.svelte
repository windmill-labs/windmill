<script lang="ts">
	import type { ChartData, ChartOptions } from 'chart.js'
	import type { HTMLCanvasAttributes } from 'svelte/elements'
	import Chart from './Chart.svelte'

  interface Props extends HTMLCanvasAttributes {
    data: ChartData<"doughnut">;
    options: ChartOptions<"doughnut">;
  }
  const props: Props = $props()
  
</script>

<Chart type="doughnut" {...props} />