<script lang="ts">
	import { getContext, untrack } from 'svelte'
	import type { AppEditorContext } from '../types'
	import TestJobLoader from '$lib/components/TestJobLoader.svelte'
	import type { Job } from '$lib/gen'
	import RunnableJobPanelInner from './RunnableJobPanelInner.svelte'

	interface Props {
		float?: boolean
		hidden?: boolean
		testJob?: Job | undefined
		jobToWatch?: { componentId: string; job: string } | undefined
		width?: number | undefined
	}

	let {
		float = true,
		hidden = false,
		testJob = $bindable(undefined),
		jobToWatch = $bindable(undefined),
		width = undefined
	}: Props = $props()

	const { runnableJobEditorPanel, selectedComponentInEditor } =
		getContext<AppEditorContext>('AppEditorContext')
	let testIsLoading = $state(false)

	let testJobLoader: TestJobLoader | undefined = $state()

	function updateSelectedJob() {
		const selectedComponent = $selectedComponentInEditor

		if (selectedComponent) {
			const backendJob = $runnableJobEditorPanel.jobs[selectedComponent]
			if (backendJob) {
				if (jobToWatch?.job == backendJob) {
					return
				}
				if (jobToWatch?.componentId != selectedComponent) {
					testJob = undefined
				}
				jobToWatch = { componentId: selectedComponent, job: backendJob }
				testJobLoader?.watchJob(backendJob)
			} else {
				testJob = undefined
				jobToWatch = undefined
			}
		}
	}

	let logDrawerOpen = false
	let resultDrawerOpen = false
	$effect(() => {
		;($runnableJobEditorPanel.focused || !float) &&
			$selectedComponentInEditor &&
			$runnableJobEditorPanel.jobs &&
			untrack(() => {
				updateSelectedJob()
			})
	})
</script>

<TestJobLoader bind:this={testJobLoader} bind:isLoading={testIsLoading} bind:job={testJob} />

{#if ($runnableJobEditorPanel.focused && $selectedComponentInEditor) || logDrawerOpen || resultDrawerOpen || !float}
	{@const frontendJob = $runnableJobEditorPanel?.frontendJobs[$selectedComponentInEditor ?? '']}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	{#if float}
		<div
			class="absolute z-[100] top-0 right-0 border-t h-full"
			style="width: {$runnableJobEditorPanel.width}px; transform: translateX({$runnableJobEditorPanel.width}px);"
		>
			<RunnableJobPanelInner {testIsLoading} {frontendJob} {testJob} />
		</div>
	{:else}
		<div
			class="flex flex-col min-w-0 grow h-full"
			style={width !== undefined ? `width:${width}px;` : ''}
		>
			{#if $selectedComponentInEditor}
				<RunnableJobPanelInner {testIsLoading} {frontendJob} {testJob} />
			{:else if !hidden}
				<div class="text-sm text-secondary text-center py-8 px-2 border-l h-full">
					Logs and results will be displayed here
				</div>
			{/if}
		</div>
	{/if}
{/if}
