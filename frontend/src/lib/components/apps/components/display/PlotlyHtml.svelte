<script lang="ts">
	import { getContext, onMount } from 'svelte'
	import { initOutput } from '../../editor/appUtils'
	import type { AppInput } from '../../inputType'
	import type { AppViewerContext } from '../../types'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	// export let configuration: Record<string, AppInput>
	export let initializing: boolean | undefined = undefined
	export let render: boolean

	const { worldStore } = getContext<AppViewerContext>('AppViewerContext')

	initOutput($worldStore, id, {
		result: undefined,
		loading: false
	})

	let result: object | undefined = undefined
	let divEl: HTMLDivElement | null = null

	let Plotly
	onMount(async () => {
		//@ts-ignore
		await import('https://cdn.plot.ly/plotly-2.18.0.min.js')

		Plotly = window['Plotly']
	})

	let h: number | undefined = undefined
	let w: number | undefined = undefined

	$: Plotly &&
		render &&
		result &&
		divEl &&
		h &&
		w &&
		Plotly.newPlot(
			divEl,
			[result],
			{ width: w, height: h, margin: { l: 50, r: 40, b: 40, t: 40, pad: 4 } },
			{ responsive: true, displayModeBar: false }
		)
</script>

<div class="w-full h-full" bind:clientHeight={h} bind:clientWidth={w}>
	<RunnableWrapper {render} flexWrap {componentInput} {id} bind:initializing bind:result>
		<div on:pointerdown bind:this={divEl} />
	</RunnableWrapper>
</div>
