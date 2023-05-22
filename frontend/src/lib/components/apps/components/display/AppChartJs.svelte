<script lang="ts">
	import { Chart } from 'svelte-chartjs'
	import { Chart as ChartJS, registerables, type ChartOptions } from 'chart.js'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import type { AppInput } from '../../inputType'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { concatCustomCss } from '../../utils'
	import { getContext } from 'svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'

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

	ChartJS.register(...registerables)

	let result: undefined = undefined

	const options = {
		responsive: true,
		animation: false,
		maintainAspectRatio: false
	} as ChartOptions

	$: css = concatCustomCss($app.css?.piechartcomponent, customCss)

	const resolvedConfig = initConfig(
		components['chartjscomponent'].initialData.configuration,
		configuration
	)
</script>

{#each Object.keys(components['chartjscomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

<RunnableWrapper {outputs} {render} autoRefresh {componentInput} {id} bind:initializing bind:result>
	<div class="w-full h-full {css?.container?.class ?? ''}" style={css?.container?.style ?? ''}>
		{#if result && resolvedConfig.type}
			{#key resolvedConfig.type}
				<Chart type={resolvedConfig.type} data={result} {options} />
			{/key}
		{/if}
	</div>
</RunnableWrapper>
