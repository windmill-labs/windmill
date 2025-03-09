<!-- https://github.com/SauravKanchan/svelte-chartjs/issues/158#issuecomment-2456212827 -->
<!-- Solves svelte-chartjs not being compatible with Svelte 5
     because it's using svelte/internal -->

<script lang="ts">
  import {
    Chart,
    Tooltip,
    type ChartData,
    type ChartOptions,
  } from 'chart.js';
  import type { HTMLCanvasAttributes } from 'svelte/elements';

  import 'chart.js/auto';
  import 'chartjs-adapter-date-fns';

  interface Props extends HTMLCanvasAttributes {
    data: ChartData<'line', any, string>;
    options: ChartOptions<'line'>;
  }

  const { data, options, ...rest }: Props = $props();

  Chart.register(Tooltip);

  let canvasElem: HTMLCanvasElement;
  let chart: Chart;

  $effect(() => {
    chart = new Chart(canvasElem, {
      type: 'line',
      data,
      options,
    });

    return () => {
      chart.destroy();
    };
  });

  $effect(() => {
    if (chart) {
      chart.data = data;
      chart.update();
    }
  });
</script>

<canvas bind:this={canvasElem} {...rest}></canvas>