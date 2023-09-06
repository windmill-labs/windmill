<script lang="ts">
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import type { AppInput } from '../../inputType'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import { initCss } from '../../utils'
	import FlowStatusViewer from '$lib/components/FlowStatusViewer.svelte'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let initializing: boolean | undefined = false
	export let customCss: ComponentCustomCSS<'jobidflowstatuscomponent'> | undefined = undefined
	export let render: boolean
	export let configuration: RichConfigurations

	const { app, worldStore, workspace } = getContext<AppViewerContext>('AppViewerContext')

	let resolvedConfig = initConfig(
		components['jobidlogcomponent'].initialData.configuration,
		configuration
	)

	const outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false
	})

	initializing = false

	let css = initCss($app.css?.jobidflowstatuscomponent, customCss)

	$: jobId = resolvedConfig.jobId
</script>

{#each Object.keys(components['jobidflowstatuscomponent'].initialData.configuration) as key (key)}
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
		componentStyle={$app.css?.jobidflowstatuscomponent}
	/>
{/each}

<RunnableWrapper {outputs} {render} {componentInput} {id}>
	<div class="flex flex-col w-full h-full">
		<div
			class={twMerge(
				'w-full border-b px-2 text-xs p-1 font-semibold bg-gray-500 text-white rounded-t-sm',
				css?.header?.class
			)}
			style={css?.header?.style}
		>
			Flow Status
		</div>
		<div
			style={twMerge(
				$app.css?.['flowstatuscomponent']?.['container']?.style,
				customCss?.container?.style
			)}
			class={twMerge(
				'p-2 grow overflow-auto',
				$app.css?.['flowstatuscomponent']?.['container']?.class,
				customCss?.container?.class
			)}
		>
			{#if jobId}
				<FlowStatusViewer workspaceId={workspace} {jobId} />
			{:else}
				<span class="text-secondary text-xs">No flow</span>
			{/if}
		</div>
	</div>
</RunnableWrapper>
