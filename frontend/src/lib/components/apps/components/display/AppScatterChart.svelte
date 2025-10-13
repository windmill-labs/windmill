<script lang="ts">
	import zoomPlugin from 'chartjs-plugin-zoom'
	import {
		Chart as ChartJS,
		Title,
		Tooltip,
		Legend,
		LineElement,
		LinearScale,
		PointElement,
		CategoryScale,
		BarElement,
		type Point
	} from 'chart.js'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import type { AppInput } from '../../inputType'
	import InputValue from '../helpers/InputValue.svelte'
	import type { ChartOptions, ChartData } from 'chart.js'
	import { initCss } from '../../utils'
	import { getContext } from 'svelte'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { initOutput } from '../../editor/appUtils'
	import { twMerge } from 'tailwind-merge'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import { Scatter } from '$lib/components/chartjs-wrappers/chartJs'

	interface Props {
		id: string
		componentInput: AppInput | undefined
		configuration: RichConfigurations
		initializing?: boolean | undefined
		customCss?: ComponentCustomCSS<'scatterchartcomponent'> | undefined
		render: boolean
	}

	let {
		id,
		componentInput,
		configuration,
		initializing = $bindable(undefined),
		customCss = undefined,
		render
	}: Props = $props()

	let zoomable = $state(false)
	let pannable = $state(false)

	const { app, worldStore } = getContext<AppViewerContext>('AppViewerContext')

	const outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false
	})

	ChartJS.register(
		Title,
		Tooltip,
		Legend,
		LineElement,
		LinearScale,
		PointElement,
		CategoryScale,
		BarElement,
		zoomPlugin
	)

	let result: { data: { x: any[]; y: string[] } } | undefined = $state(undefined)

	let options = $derived({
		responsive: true,
		animation: false,
		maintainAspectRatio: false,
		plugins: {
			zoom: {
				pan: {
					enabled: pannable
				},
				zoom: {
					drag: {
						enabled: false
					},
					wheel: {
						enabled: zoomable
					}
				}
			}
		}
	} as ChartOptions<'scatter'>)

	let data = $derived({
		datasets: result ?? []
	} as ChartData<'scatter', (number | Point)[], unknown>)

	let css = $state(initCss($app.css?.scatterchartcomponent, customCss))
</script>

{#each Object.keys(css ?? {}) as key (key)}
	<ResolveStyle
		{id}
		{customCss}
		{key}
		bind:css={css[key]}
		componentStyle={$app.css?.scatterchartcomponent}
	/>
{/each}

<InputValue key="zoomable" {id} input={configuration.zoomable} bind:value={zoomable} />
<InputValue key="pannable" {id} input={configuration.pannable} bind:value={pannable} />

<RunnableWrapper {outputs} {render} autoRefresh {componentInput} {id} bind:initializing bind:result>
	<div
		class={twMerge('w-full h-full', css?.container?.class, 'wm-scatter-chart')}
		style={css?.container?.style ?? ''}
	>
		{#if result}
			<Scatter {data} {options} />
		{/if}
	</div>
</RunnableWrapper>
