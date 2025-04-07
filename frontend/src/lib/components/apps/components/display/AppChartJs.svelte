<script lang="ts">
	import { Chart } from '$lib/components/chartjs-wrappers/chartJs'
	import { Chart as ChartJS, registerables, type ChartOptions } from 'chart.js'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import type { AppInput } from '../../inputType'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { initCss } from '../../utils'
	import { getContext } from 'svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import { twMerge } from 'tailwind-merge'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: RichConfigurations
	export let initializing: boolean | undefined = undefined
	export let customCss: ComponentCustomCSS<'chartjscomponent'> | undefined = undefined
	export let render: boolean

	const { app, worldStore } = getContext<AppViewerContext>('AppViewerContext')

	const outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false
	})

	ChartJS.register(...registerables)

	let result: undefined = undefined

	const resolvedConfig = initConfig(
		components['chartjscomponent'].initialData.configuration,
		configuration
	)
	$: options = {
		responsive: true,
		animation: false,
		maintainAspectRatio: false,
		...(resolvedConfig.options ?? {})
	} as ChartOptions

	let css = initCss($app.css?.chartjscomponent, customCss)
</script>

{#each Object.keys(components['chartjscomponent'].initialData.configuration) as key (key)}
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
		componentStyle={$app.css?.chartjscomponent}
	/>
{/each}

<RunnableWrapper {outputs} {render} autoRefresh {componentInput} {id} bind:initializing bind:result>
	<div
		class={twMerge('w-full h-full', css?.container?.class, 'wm-chartjs')}
		style={css?.container?.style ?? ''}
	>
		{#if result && resolvedConfig.type}
			{#key resolvedConfig.type}
				{#key options}
					<Chart type={resolvedConfig.type} data={result} {options} />
				{/key}
			{/key}
		{/if}
	</div>
</RunnableWrapper>
