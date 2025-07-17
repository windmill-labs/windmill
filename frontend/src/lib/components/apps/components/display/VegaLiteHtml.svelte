<script lang="ts">
	import { createBubbler } from 'svelte/legacy'

	const bubble = createBubbler()
	import { getContext, onMount } from 'svelte'
	import { initOutput } from '../../editor/appUtils'
	import type { AppInput } from '../../inputType'
	import type { AppViewerContext, RichConfigurations } from '../../types'
	import InputValue from '../helpers/InputValue.svelte'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'

	interface Props {
		id: string
		componentInput: AppInput | undefined
		configuration: RichConfigurations
		initializing?: boolean | undefined
		render: boolean
	}

	let {
		id,
		componentInput,
		configuration,
		initializing = $bindable(undefined),
		render
	}: Props = $props()

	let result: object | undefined = $state(undefined)
	let divEl: HTMLDivElement | null = $state(null)

	const { worldStore } = getContext<AppViewerContext>('AppViewerContext')

	const outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false
	})

	let vegaEmbed = $state() as any
	onMount(async () => {
		//@ts-ignore
		await import(/* webpackIgnore: true */ 'https://cdn.jsdelivr.net/npm/vega@5.22.1')
		//@ts-ignore
		await import(/* webpackIgnore: true */ 'https://cdn.jsdelivr.net/npm/vega-lite@5.6.0')
		//@ts-ignore
		await import(/* webpackIgnore: true */ 'https://cdn.jsdelivr.net/npm/vega-embed@6.21.0')

		vegaEmbed = window['vegaEmbed']
	})

	let h: number | undefined = $state(undefined)
	let w: number | undefined = $state(undefined)
	let canvas = $state(false)

	$effect(() => {
		vegaEmbed &&
			result &&
			divEl &&
			h &&
			w &&
			vegaEmbed(
				divEl,
				{
					...result,
					...{
						width: w - 100,
						config: {
							legend: { orient: 'bottom', ...(result?.['config']?.['legend'] ?? {}) },
							...(result?.['config'] ?? {})
						}
					}
				},
				{
					mode: 'vega-lite',
					actions: false,
					renderer: canvas ? 'canvas' : 'svg',
					height: h - 75,
					width: w - 100
				}
			)
	})
</script>

<InputValue key={'canvas'} {id} input={configuration.canvas} bind:value={canvas} />

{#if render}
	<div class="w-full h-full" bind:clientHeight={h} bind:clientWidth={w}>
		<RunnableWrapper {outputs} {render} {componentInput} {id} bind:initializing bind:result>
			<div onpointerdown={bubble('pointerdown')} bind:this={divEl}></div>
		</RunnableWrapper>
	</div>
{:else}
	<RunnableWrapper {outputs} {render} {componentInput} {id} />
{/if}
