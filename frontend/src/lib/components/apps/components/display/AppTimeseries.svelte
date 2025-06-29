<script lang="ts">
	import zoomPlugin from 'chartjs-plugin-zoom'
	import 'chartjs-adapter-date-fns'
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
		TimeScale,
		LogarithmicScale,
		type Point
	} from 'chart.js'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import type { AppInput } from '../../inputType'
	import { Scatter } from '$lib/components/chartjs-wrappers/chartJs'
	import InputValue from '../helpers/InputValue.svelte'
	import type { ChartOptions, ChartData } from 'chart.js'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { getContext } from 'svelte'
	import { initCss } from '../../utils'
	import { initOutput } from '../../editor/appUtils'
	import { twMerge } from 'tailwind-merge'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'

	interface Props {
		id: string
		componentInput: AppInput | undefined
		configuration: RichConfigurations
		initializing?: boolean | undefined
		customCss?: ComponentCustomCSS<'timeseriescomponent'> | undefined
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

	const { app, worldStore } = getContext<AppViewerContext>('AppViewerContext')

	const outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false
	})

	let logarithmicScale = $state(false)
	let zoomable = $state(false)
	let pannable = $state(false)

	ChartJS.register(
		Title,
		Tooltip,
		Legend,
		LineElement,
		LinearScale,
		PointElement,
		CategoryScale,
		BarElement,
		zoomPlugin,
		TimeScale,
		LogarithmicScale
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
		},
		scales: {
			x: {
				type: 'time'
			},
			y: {
				type: logarithmicScale ? 'logarithmic' : 'linear'
			}
		}
	} as ChartOptions<'scatter'>)

	let data = $derived({
		datasets: result ?? []
	} as ChartData<'scatter', (number | Point)[], unknown>)

	let css = $state(initCss($app.css?.timeseriescomponent, customCss))
</script>

{#each Object.keys(css ?? {}) as key (key)}
	<ResolveStyle
		{id}
		{customCss}
		{key}
		bind:css={css[key]}
		componentStyle={$app.css?.timeseriescomponent}
	/>
{/each}

<InputValue
	key="logarithmicScale"
	{id}
	input={configuration.logarithmicScale}
	bind:value={logarithmicScale}
/>
<InputValue key="zoomable" {id} input={configuration.zoomable} bind:value={zoomable} />
<InputValue key="pannable" {id} input={configuration.pannable} bind:value={pannable} />

<RunnableWrapper {outputs} {render} autoRefresh {componentInput} {id} bind:initializing bind:result>
	<div
		class={twMerge('w-full h-full', css?.container?.class, 'wm-timeseries')}
		style={css?.container?.style ?? ''}
	>
		{#if result}
			<Scatter {data} {options} />
		{/if}
	</div>
</RunnableWrapper>
