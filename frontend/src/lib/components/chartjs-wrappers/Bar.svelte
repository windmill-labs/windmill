<script lang="ts">
	import type { ChartData, ChartOptions } from 'chart.js'
	import type { HTMLCanvasAttributes } from 'svelte/elements'
	import Chart from './Chart.svelte'

  interface Props extends HTMLCanvasAttributes {
    data: ChartData<"bar">;
    options: ChartOptions<"bar">;
  }
  const {data, options}: Props = $props()
  
</script>

<Chart type="bar" data={data} options={options} />