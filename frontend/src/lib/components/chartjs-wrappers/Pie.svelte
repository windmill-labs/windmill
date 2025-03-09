<script lang="ts">
	import type { ChartData, ChartOptions } from 'chart.js'
	import type { HTMLCanvasAttributes } from 'svelte/elements'
	import Chart from './Chart.svelte'

  interface Props extends HTMLCanvasAttributes {
    data: ChartData<"pie">;
    options: ChartOptions<"pie">;
  }
  const {data, options}: Props = $props()
  
</script>

<Chart type="pie" data={data} options={options} />