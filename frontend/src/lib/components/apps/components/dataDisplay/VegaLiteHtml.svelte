<script lang="ts">
	import { Loader2 } from 'lucide-svelte'
	import { parse } from 'path'
	import { onMount } from 'svelte'
	import type { AppInput } from '../../inputType'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InputValue from '../helpers/InputValue.svelte'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: Record<string, AppInput>

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
	<RunnableWrapper flexWrap bind:componentInput {id} bind:result>
		{#if !vegaEmbed}
			<div class="p-2">
				<Loader2 class="animate-spin" />
			</div>
		{/if}
		<div on:pointerdown bind:this={divEl} />
	</RunnableWrapper>
</div>
