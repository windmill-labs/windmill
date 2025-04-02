<script lang="ts">
	import type { ChartOptions } from 'chart.js'
	import {
		BarElement,
		CategoryScale,
		Chart as ChartJS,
		Legend,
		LinearScale,
		LineElement,
		PointElement,
		Title,
		Tooltip
	} from 'chart.js'
	import { getContext } from 'svelte'
	import { Bar, Line } from '$lib/components/chartjs-wrappers/chartJs'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import type { AppInput } from '../../inputType'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { initCss } from '../../utils'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import { twMerge } from 'tailwind-merge'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: RichConfigurations
	export let initializing: boolean | undefined = undefined
	export let customCss: ComponentCustomCSS<'barchartcomponent'> | undefined = undefined
	export let render: boolean

	const { app, worldStore } = getContext<AppViewerContext>('AppViewerContext')

	let resolvedConfig = initConfig(
		components['barchartcomponent'].initialData.configuration,
		configuration
	)

	let outputs = initOutput($worldStore, id, {
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
		BarElement
	)

	let result: { data: number[]; labels?: string[] } | undefined = undefined

	$: backgroundColor = {
		theme1: ['#FF6384', '#4BC0C0', '#FFCE56', '#E7E9ED', '#36A2EB'],
		// blue theme
		theme2: ['#4e73df', '#1cc88a', '#36b9cc', '#f6c23e', '#e74a3b'],
		// red theme
		theme3: ['#e74a3b', '#4e73df', '#1cc88a', '#36b9cc', '#f6c23e']
	}[resolvedConfig.theme ?? 'theme1']

	const lineOptions: ChartOptions<'line'> = {
		responsive: true,
		animation: false,
		maintainAspectRatio: false,
		plugins: {
			legend: {
				display: false
			}
		}
	}

	const barOptions: ChartOptions<'bar'> = {
		responsive: true,
		animation: false,
		maintainAspectRatio: false,
		plugins: {
			legend: {
				display: false
			}
		}
	}

	$: data = {
		labels: result?.labels ?? [],

		datasets: [
			{
				data: result?.data ?? [],
				backgroundColor
			}
		],
		options: {
			scales: {
				y: {
					ticks: {
						// Include a dollar sign in the ticks
						callback: function (value, index, ticks) {
							return '$' + value
						}
					}
				}
			}
		}
	}

	let css = initCss($app.css?.barchartcomponent, customCss)
</script>

{#each Object.keys(components['barchartcomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

{#each Object.keys(css ?? {}) as key (key)}
	<ResolveStyle
		{id}
		{customCss}
		{key}
		bind:css={css[key]}
		componentStyle={$app.css?.barchartcomponent}
	/>
{/each}

<RunnableWrapper {outputs} {render} autoRefresh {componentInput} {id} bind:initializing bind:result>
	<div
		class={twMerge('w-full h-full', css?.container?.class, 'wm-bar-chart')}
		style={css?.container?.style ?? ''}
	>
		{#if result}
			{#if resolvedConfig.line}
				<Line {data} options={lineOptions} />
			{:else}
				<Bar {data} options={barOptions} />
			{/if}
		{/if}
	</div>
</RunnableWrapper>
