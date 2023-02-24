<script lang="ts">
	import { Doughnut, Pie } from 'svelte-chartjs'
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
	import type { AppEditorContext, ComponentCustomCSS } from '../../types'
	import { concatCustomCss } from '../../utils'
	import { getContext } from 'svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: Record<string, AppInput>
	export let initializing: boolean | undefined = undefined
	export let customCss: ComponentCustomCSS<'container'> | undefined = undefined

	export const staticOutputs: string[] = ['loading', 'result']
	const { app } = getContext<AppEditorContext>('AppEditorContext')

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

	$: css = concatCustomCss($app.css?.piechartcomponent, customCss)
</script>

<InputValue {id} input={configuration.theme} bind:value={theme} />
<InputValue {id} input={configuration.doughnutStyle} bind:value={doughnut} />

<RunnableWrapper flexWrap autoRefresh bind:componentInput {id} bind:initializing bind:result>
	<div class="w-full h-full {css?.container.class ?? ''}" style={css?.container.style ?? ''}>
		{#if result}
			{#if doughnut}
				<Doughnut {data} {options} />
			{:else}
				<Pie {data} {options} />
			{/if}
		{/if}
	</div>
</RunnableWrapper>
