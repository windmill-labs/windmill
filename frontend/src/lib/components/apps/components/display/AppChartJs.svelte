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

	interface Props {
		id: string
		componentInput: AppInput | undefined
		configuration: RichConfigurations
		initializing?: boolean | undefined
		customCss?: ComponentCustomCSS<'chartjscomponent'> | undefined
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

	ChartJS.register(...registerables)

	let result: undefined = $state(undefined)

	const resolvedConfig = $state(
		initConfig(components['chartjscomponent'].initialData.configuration, configuration)
	)
	let options = $derived({
		responsive: true,
		animation: false,
		maintainAspectRatio: false,
		...(resolvedConfig.options ?? {})
	} as ChartOptions)

	let css = $state(initCss($app.css?.chartjscomponent, customCss))
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
