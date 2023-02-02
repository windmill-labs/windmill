<script lang="ts">
	import { Loader2 } from 'lucide-svelte'
	import { onMount } from 'svelte'
	import type { AppInput } from '../../inputType'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: Record<string, AppInput>
	export let initializing

	export const staticOutputs: string[] = ['result', 'loading']

	let result: object | undefined = undefined
	let divEl: HTMLDivElement | null = null

	let Plotly
	onMount(async () => {
		if (divEl) {
			//@ts-ignore
			await import('https://cdn.plot.ly/plotly-2.18.0.min.js')

			Plotly = window['Plotly']
		}
	})

	let h: number | undefined = undefined
	let w: number | undefined = undefined

	$: Plotly &&
		result &&
		divEl &&
		h &&
		w &&
		Plotly.newPlot(
			divEl,
			[result],
			{ width: w, height: h, margin: { l: 40, r: 40, b: 40, t: 40, pad: 4 } },
			{ responsive: true, displayModeBar: false }
		)
</script>

<div class="w-full h-full" bind:clientHeight={h} bind:clientWidth={w}>
	<RunnableWrapper flexWrap bind:componentInput {id} bind:initializing bind:result>
		{#if !Plotly}
			<div class="p-2">
				<Loader2 class="animate-spin" />
			</div>
		{/if}
		<div on:pointerdown bind:this={divEl} />
	</RunnableWrapper>
</div>
