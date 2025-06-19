<script lang="ts">
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { initCss } from '../../utils'
	import LogViewer from '$lib/components/LogViewer.svelte'
	import TestJobLoader from '$lib/components/TestJobLoader.svelte'
	import type { Job } from '$lib/gen'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'

	export let id: string
	export let initializing: boolean | undefined = false
	export let customCss: ComponentCustomCSS<'jobidlogcomponent'> | undefined = undefined
	export let configuration: RichConfigurations
	export let render: boolean

	const { app, worldStore, workspace } = getContext<AppViewerContext>('AppViewerContext')

	let resolvedConfig = initConfig(
		components['jobidlogcomponent'].initialData.configuration,
		configuration
	)

	const outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false,
		jobId: undefined
	})

	initializing = false

	let css = initCss(app.val.css?.jobidlogcomponent, customCss)

	let testJobLoader: TestJobLoader | undefined = undefined
	let testIsLoading: boolean = false
	let testJob: Job | undefined = undefined

	$: if (resolvedConfig.jobId) {
		outputs.loading.set(true)
		testJobLoader?.watchJob(resolvedConfig?.['jobId'])
	}
</script>

{#each Object.keys(components['jobidlogcomponent'].initialData.configuration) as key (key)}
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
		componentStyle={app.val.css?.jobidlogcomponent}
	/>
{/each}

<TestJobLoader
	workspaceOverride={workspace}
	bind:this={testJobLoader}
	bind:isLoading={testIsLoading}
	bind:job={testJob}
	on:done={(e) => {
		outputs.loading.set(false)
		outputs.jobId.set(e.detail.id)
		outputs.result.set(e.detail.result)
	}}
/>

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
			Logs
		</div>
		<div
			style={css?.container?.style}
			class={twMerge('grow overflow-auto', css?.container?.class, 'wm-log-container')}
		>
			<LogViewer
				jobId={testJob?.id}
				duration={testJob?.['duration_ms']}
				mem={testJob?.['mem_peak']}
				content={testJob?.logs}
				isLoading={testIsLoading && testJob?.['running'] == false}
				tag={testJob?.tag}
			/>
		</div>
	</div>
{/if}
