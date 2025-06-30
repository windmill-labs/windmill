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

	interface Props {
		id: string
		componentInput: AppInput | undefined
		configuration: RichConfigurations
		initializing?: boolean | undefined
		customCss?: ComponentCustomCSS<'piechartcomponent'> | undefined
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

	let result = $state(undefined) as { data: number[]; labels?: string[] } | undefined
	let theme: string = $state('theme1')
	let doughnut = $state(false)

	let backgroundColor = $derived(
		{
			theme1: ['#FF6384', '#4BC0C0', '#FFCE56', '#E7E9ED', '#36A2EB'],
			// blue theme
			theme2: ['#4e73df', '#1cc88a', '#36b9cc', '#f6c23e', '#e74a3b'],
			// red theme
			theme3: ['#e74a3b', '#4e73df', '#1cc88a', '#36b9cc', '#f6c23e']
		}[theme]
	)

	const options = {
		responsive: true,
		animation: false,
		maintainAspectRatio: false
	}

	let data = $derived({
		labels: result?.labels ?? [],
		datasets: [
			{
				data: result?.data ?? [],
				backgroundColor: backgroundColor
			}
		]
	})

	let css = $state(initCss($app.css?.piechartcomponent, customCss))
</script>

{#each Object.keys(css ?? {}) as key (key)}
	<ResolveStyle
		{id}
		{customCss}
		{key}
		bind:css={css[key]}
		componentStyle={$app.css?.piechartcomponent}
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
