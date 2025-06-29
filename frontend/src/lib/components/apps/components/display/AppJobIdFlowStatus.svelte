<script lang="ts">
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { initCss } from '../../utils'
	import FlowStatusViewer from '$lib/components/FlowStatusViewer.svelte'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'

	interface Props {
		id: string
		initializing?: boolean | undefined
		customCss?: ComponentCustomCSS<'jobidflowstatuscomponent'> | undefined
		configuration: RichConfigurations
		render: boolean
	}

	let {
		id,
		initializing = $bindable(undefined),
		customCss = undefined,
		configuration,
		render
	}: Props = $props()

	$effect.pre(() => {
		if (initializing) {
			initializing = false
		}
	})

	const { app, worldStore, workspace } = getContext<AppViewerContext>('AppViewerContext')

	const resolvedConfig = $state(
		initConfig(components['jobidlogcomponent'].initialData.configuration, configuration)
	)

	const outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false,
		jobId: undefined as string | undefined
	})

	initializing = false

	let css = $state(initCss($app.css?.jobidflowstatuscomponent, customCss))

	let jobId = $derived(resolvedConfig.jobId)
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

<InitializeComponent {id} />

{#if render}
	<div class="flex flex-col w-full h-full component-wrapper">
		<div
			class={twMerge(
				'w-full border-b p-2 text-xs font-semibold text-primary bg-surface-secondary',
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
				<FlowStatusViewer
					workspaceId={workspace}
					{jobId}
					on:start={() => {
						outputs?.jobId.set(jobId)
						outputs?.loading.set(true)
					}}
					on:done={(e) => {
						outputs?.loading.set(false)
						outputs?.result.set(e?.detail?.result)
					}}
				/>
			{:else}
				<span class="text-secondary text-xs">No flow</span>
			{/if}
		</div>
	</div>
{/if}
