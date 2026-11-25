<!-- https://github.com/SauravKanchan/svelte-chartjs/issues/158#issuecomment-2456212827 -->
<!-- Replaces svelte-chartjs not working with Svelte 5 because it's using svelte/internal -->

<script lang="ts" generics="T extends ChartType">
	import { Chart, Tooltip, type ChartData, type ChartOptions, type ChartType } from 'chart.js'
	import type { HTMLCanvasAttributes } from 'svelte/elements'

	import 'chart.js/auto'
	import 'chartjs-adapter-date-fns'
	import { untrack } from 'svelte'

	interface Props extends HTMLCanvasAttributes {
		type: T
		data: ChartData<T>
		options: ChartOptions<T>
	}

	const { type, data, options, ...rest }: Props = $props()

	Chart.register(Tooltip)

	let canvasElem: HTMLCanvasElement
	let chart: Chart<T>

	// Options can be $state proxies (app components), so chart.js must get a plain copy.
	// $state.snapshot would do it but warns in dev for every callback it meets.
	function cloneOptions(value: unknown, seen = new Map<object, unknown>()): unknown {
		if (typeof value !== 'object' || value === null) return value
		if (seen.has(value)) return seen.get(value)
		if (Array.isArray(value)) {
			const copy: unknown[] = []
			seen.set(value, copy)
			for (const item of value) copy.push(cloneOptions(item, seen))
			return copy
		}
		const proto = Object.getPrototypeOf(value)
		if (proto !== Object.prototype && proto !== null) return value
		const source = value as Record<string, unknown>
		const copy: Record<string, unknown> = {}
		seen.set(value, copy)
		for (const key of Object.keys(source)) copy[key] = cloneOptions(source[key], seen)
		return copy
	}

	$effect(() => {
		;[type, data, options]
		untrack(() => {
			chart = new Chart(canvasElem, {
				type,
				data: $state.snapshot(data) as typeof data,
				options: cloneOptions(options) as typeof options
			})
		})
		return () => {
			chart.destroy()
		}
	})

	$effect(() => {
		data
		untrack(() => {
			if (chart) {
				chart.data = $state.snapshot(data) as typeof data
				chart.update()
			}
		})
	})
</script>

<canvas bind:this={canvasElem} {...rest}></canvas>
