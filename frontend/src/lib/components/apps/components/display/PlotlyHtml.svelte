<script lang="ts">
	import { createBubbler } from 'svelte/legacy'

	const bubble = createBubbler()
	import { Alert } from '$lib/components/common'
	import { getContext, onMount, untrack } from 'svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import { components } from '../../editor/component'
	import type { AppInput } from '../../inputType'
	import type { AppViewerContext, RichConfigurations } from '../../types'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
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

	const { worldStore, darkMode } = getContext<AppViewerContext>('AppViewerContext')

	let resolvedConfig = $state(
		initConfig(components['plotlycomponent'].initialData.configuration, configuration)
	)

	const outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false
	})

	let result: object | undefined = $state(undefined)
	let divEl: HTMLDivElement | null = $state(null)

	let Plotly = $state() as any
	onMount(async () => {
		//@ts-ignore
		await import(
			/* @vite-ignore */
			/* webpackIgnore: true */
			//@ts-ignore
			'https://cdn.jsdelivr.net/npm/plotly.js-dist@2.18.0/plotly.min.js'
		)

		Plotly = window['Plotly']
	})

	let h: number | undefined = $state(undefined)
	let w: number | undefined = $state(undefined)

	let shouldeUpdate = $state(1)

	darkMode.subscribe(() => {
		shouldeUpdate++
	})

	let error = $state('')
	function plot() {
		try {
			Plotly.newPlot(
				divEl,
				Array.isArray(result) ? result : [result],
				{
					width: w,
					height: h,
					margin: { l: 50, r: 40, b: 40, t: 40, pad: 4 },
					paper_bgcolor: $darkMode ? '#2e3440' : '#fff',
					plot_bgcolor: $darkMode ? '#2e3440' : '#fff',
					...resolvedConfig.layout,
					xaxis: {
						color: $darkMode ? '#f3f6f8' : '#000',
						...(resolvedConfig?.layout?.['xaxis'] ?? {})
					},
					yaxis: {
						color: $darkMode ? '#f3f6f8' : '#000',
						...(resolvedConfig?.layout?.['yaxis'] ?? {})
					}
				},
				{ responsive: true, displayModeBar: false }
			)
			error = ''
		} catch (e) {
			error = e.message
			console.error(e)
		}
	}
	$effect.pre(() => {
		Plotly &&
			render &&
			result &&
			resolvedConfig.layout &&
			divEl &&
			h &&
			w &&
			shouldeUpdate &&
			untrack(() => plot())
	})
</script>

{#each Object.keys(components['plotlycomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

{#if render}
	<div class="w-full h-full" bind:clientHeight={h} bind:clientWidth={w}>
		<RunnableWrapper {outputs} {render} {componentInput} {id} bind:initializing bind:result>
			{#if error != ''}
				<div class="flex flex-col h-full w-full overflow-auto">
					<Alert title="Plotly error" type="error" size="xs" class="h-full w-full ">
						<pre class="w-full bg-surface p-2 rounded-md whitespace-pre-wrap">{error}</pre>
					</Alert>
				</div>
			{/if}
			<div onpointerdown={bubble('pointerdown')} bind:this={divEl}></div>
		</RunnableWrapper>
	</div>
{:else}
	<RunnableWrapper {outputs} {render} {componentInput} {id} />
{/if}
