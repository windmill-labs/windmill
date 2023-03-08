<script lang="ts">
	import { onMount } from 'svelte'
	import type { AppInput } from '../../inputType'
	import InputValue from '../helpers/InputValue.svelte'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: Record<string, AppInput>
	export let initializing: boolean | undefined = undefined
	export let render: boolean

	export const staticOutputs: string[] = ['result', 'loading']

	let result: object | undefined = undefined
	let divEl: HTMLDivElement | null = null

	let vegaEmbed
	onMount(async () => {
		if (divEl) {
			//@ts-ignore
			await import('https://cdn.jsdelivr.net/npm/vega@5.22.1')
			//@ts-ignore
			await import('https://cdn.jsdelivr.net/npm/vega-lite@5.6.0')
			//@ts-ignore
			await import('https://cdn.jsdelivr.net/npm/vega-embed@6.21.0')
			vegaEmbed = window['vegaEmbed']
		}
	})

	let h: number | undefined = undefined
	let w: number | undefined = undefined
	let canvas = false

	$: vegaEmbed &&
		result &&
		divEl &&
		h &&
		w &&
		vegaEmbed(
			divEl,
			{ ...result, ...{ width: w - 100 } },
			{
				mode: 'vega-lite',
				actions: false,
				renderer: canvas ? 'canvas' : 'svg',
				height: h - 75,
				width: w - 75
			}
		)
</script>

<InputValue {id} input={configuration.canvas} bind:value={canvas} />

<div class="w-full h-full" bind:clientHeight={h} bind:clientWidth={w}>
	<RunnableWrapper {render} flexWrap {componentInput} {id} bind:initializing bind:result>
		<div on:pointerdown bind:this={divEl} />
	</RunnableWrapper>
</div>
