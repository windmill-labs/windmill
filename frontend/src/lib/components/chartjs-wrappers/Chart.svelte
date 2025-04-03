<!-- https://github.com/SauravKanchan/svelte-chartjs/issues/158#issuecomment-2456212827 -->
<!-- Replaces svelte-chartjs not working with Svelte 5 because it's using svelte/internal -->

<script lang="ts" generics="T extends ChartType">
  import {
    Chart,
    Tooltip,
    type ChartData,
    type ChartOptions,
	type ChartType,
  } from 'chart.js';
  import type { HTMLCanvasAttributes } from 'svelte/elements';

  import 'chart.js/auto';
  import 'chartjs-adapter-date-fns';

  interface Props extends HTMLCanvasAttributes {
    type: T,
    data: ChartData<T>;
    options: ChartOptions<T>;
  }

  const { type, data, options, ...rest }: Props = $props();

  Chart.register(Tooltip);

  let canvasElem: HTMLCanvasElement;
  let chart: Chart<T>;

  $effect(() => {
    chart = new Chart(canvasElem, {
      type,
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