<script lang="ts">
	import { getContext, untrack } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { initCss } from '../../utils'
	import LogViewer from '$lib/components/LogViewer.svelte'
	import JobLoader from '$lib/components/JobLoader.svelte'
	import type { Job } from '$lib/gen'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'

	interface Props {
		id: string
		initializing?: boolean | undefined
		customCss?: ComponentCustomCSS<'jobidlogcomponent'> | undefined
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

	let resolvedConfig = $state(
		initConfig(components['jobidlogcomponent'].initialData.configuration, configuration)
	)

	const outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false,
		jobId: undefined
	})

	initializing = false

	let css = $state(initCss($app.css?.jobidlogcomponent, customCss))

	let jobLoader: JobLoader | undefined = $state(undefined)
	let testIsLoading: boolean = $state(false)
	let testJob: Job | undefined = $state(undefined)

	$effect(() => {
		if (resolvedConfig.jobId) {
			untrack(() => {
				outputs.loading.set(true)
				const jobId = resolvedConfig?.['jobId']
				if (jobId) {
					jobLoader?.watchJob(jobId)
				}
			})
		}
	})
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
		componentStyle={$app.css?.jobidlogcomponent}
	/>
{/each}

<JobLoader
	noCode={true}
	workspaceOverride={workspace}
	bind:this={jobLoader}
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
