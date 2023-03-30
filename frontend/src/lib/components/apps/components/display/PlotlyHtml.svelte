<script lang="ts">
	import { Alert } from '$lib/components/common'
	import { getContext, onMount } from 'svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import { components } from '../../editor/component'
	import type { AppInput } from '../../inputType'
	import type { AppViewerContext, RichConfigurations } from '../../types'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: RichConfigurations

	export let initializing: boolean | undefined = undefined
	export let render: boolean

	const { worldStore } = getContext<AppViewerContext>('AppViewerContext')

	let resolvedConfig = initConfig(
		components['plotlycomponent'].initialData.configuration,
		configuration
	)

	const outputs = initOutput($worldStore, id, {
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

	$: Plotly && render && result && divEl && h && w && plot()

	let error = ''
	function plot() {
		try {
			Plotly.newPlot(
				divEl,
				Array.isArray(result) ? result : [result],
				{
					width: w,
					height: h,
					margin: { l: 50, r: 40, b: 40, t: 40, pad: 4 },
					...resolvedConfig.layout
				},
				{ responsive: true, displayModeBar: false }
			)
			error = ''
		} catch (e) {
			error = e.message
			console.error(e)
		}
	}
</script>

{#each Object.keys(components['plotlycomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

<div class="w-full h-full" bind:clientHeight={h} bind:clientWidth={w}>
	<RunnableWrapper {outputs} {render} {componentInput} {id} bind:initializing bind:result>
		{#if error != ''}
			<div class="flex flex-col h-full w-full overflow-auto">
				<Alert title="Plotly error" type="error" size="xs" class="h-full w-full ">
					<pre class="w-full bg-white p-2 rounded-md whitespace-pre-wrap">{error}</pre>
				</Alert>
			</div>
		{/if}
		<div on:pointerdown bind:this={divEl} />
	</RunnableWrapper>
</div>
