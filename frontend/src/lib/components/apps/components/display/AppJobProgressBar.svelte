<script lang="ts">
	import { getContext, untrack } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { initCss } from '../../utils'
	import JobLoader from '$lib/components/JobLoader.svelte'
	import JobProgressBar from '$lib/components/jobs/JobProgressBar.svelte'
	import type { Job } from '$lib/gen'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import FlowProgressBar from '$lib/components/flows/FlowProgressBar.svelte'

	interface Props {
		id: string
		initializing?: boolean | undefined
		customCss?: ComponentCustomCSS<'jobprogressbarcomponent'> | undefined
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
		initConfig(components['jobprogressbarcomponent'].initialData.configuration, configuration)
	)

	const outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false,
		jobId: undefined
	})

	initializing = false

	let css = $state(initCss($app.css?.jobprogressbarcomponent, customCss))

	let jobLoader: JobLoader | undefined = $state(undefined)
	let testIsLoading: boolean = $state(false)
	let testJob: Job | undefined = $state(undefined)
	let scriptProgress: number | undefined = $state(undefined)

	$effect(() => {
		if (resolvedConfig.jobId) {
			untrack(() => {
				outputs.loading.set(true)
				const jobId = resolvedConfig?.['jobId']
				if (jobId) {
					let callbacks = {
						done(x) {
							outputs.loading.set(false)
							outputs.jobId.set(x.id)
							outputs.result.set(x.result)
						}
					}
					jobLoader?.watchJob(jobId, callbacks)
				}
			})
		}
	})
</script>

{#each Object.keys(components['jobprogressbarcomponent'].initialData.configuration) as key (key)}
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
		componentStyle={$app.css?.jobprogressbarcomponent}
	/>
{/each}

<JobLoader
	noCode={true}
	workspaceOverride={workspace}
	bind:this={jobLoader}
	bind:isLoading={testIsLoading}
	bind:job={testJob}
	bind:scriptProgress
/>

<InitializeComponent {id} />

{#if render}
	<div class="flex flex-col w-full h-full component-wrapper">
		<div
			style={css?.container?.style}
			class={twMerge('p-2 grow overflow-auto', css?.container?.class)}
		>
			{#if testJob}
				{#if testJob.job_kind == 'flow' || testJob.job_kind == 'flowpreview'}
					<FlowProgressBar
						job={testJob}
						bind:currentSubJobProgress={scriptProgress}
						class="py-4 max-w-7xl mx-auto px-4"
					/>
				{:else}
					<JobProgressBar hideStepTitle job={testJob} {scriptProgress} />
				{/if}
			{:else}
				<span class="text-secondary text-xs">No job</span>
			{/if}
		</div>
	</div>
{/if}
