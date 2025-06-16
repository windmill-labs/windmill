<script lang="ts">
	import { Doughnut, Pie } from '$lib/components/chartjs-wrappers/chartJs'
	import {
		Chart as ChartJS,
		Title,
		Tooltip,
		Legend,
		LineElement,
		LinearScale,
		PointElement,
		CategoryScale,
		ArcElement
	} from 'chart.js'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import type { AppInput } from '../../inputType'
	import InputValue from '../helpers/InputValue.svelte'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { initCss } from '../../utils'
	import { getContext } from 'svelte'
	import { initOutput } from '../../editor/appUtils'
	import { twMerge } from 'tailwind-merge'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: RichConfigurations
	export let initializing: boolean | undefined = undefined
	export let customCss: ComponentCustomCSS<'piechartcomponent'> | undefined = undefined
	export let render: boolean

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
		ArcElement
	)

	let result: { data: number[]; labels?: string[] } | undefined = undefined
	let theme: string = 'theme1'
	let doughnut = false

	$: backgroundColor = {
		theme1: ['#FF6384', '#4BC0C0', '#FFCE56', '#E7E9ED', '#36A2EB'],
		// blue theme
		theme2: ['#4e73df', '#1cc88a', '#36b9cc', '#f6c23e', '#e74a3b'],
		// red theme
		theme3: ['#e74a3b', '#4e73df', '#1cc88a', '#36b9cc', '#f6c23e']
	}[theme]

	const options = {
		responsive: true,
		animation: false,
		maintainAspectRatio: false
	}

	$: data = {
		labels: result?.labels ?? [],
		datasets: [
			{
				data: result?.data ?? [],
				backgroundColor: backgroundColor
			}
		]
	}

	let css = initCss(app.val.css?.piechartcomponent, customCss)
</script>

{#each Object.keys(css ?? {}) as key (key)}
	<ResolveStyle
		{id}
		{customCss}
		{key}
		bind:css={css[key]}
		componentStyle={app.val.css?.piechartcomponent}
	/>
{/each}

<InputValue key="theme" {id} input={configuration.theme} bind:value={theme} />
<InputValue key="doughnut" {id} input={configuration.doughnutStyle} bind:value={doughnut} />

<RunnableWrapper {outputs} {render} autoRefresh {componentInput} {id} bind:initializing bind:result>
	<div
		class={twMerge('w-full h-full', css?.container?.class, 'wm-pie-chart')}
		style={css?.container?.style ?? ''}
	>
		{#if result}
			{#if doughnut}
				<Doughnut {data} {options} />
			{:else}
				<Pie {data} {options} />
			{/if}
		{/if}
	</div>
</RunnableWrapper>
