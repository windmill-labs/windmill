<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '../types'
	import TestJobLoader from '$lib/components/TestJobLoader.svelte'
	import type { Job } from '$lib/gen'
	import RunnableJobPanelInner from './RunnableJobPanelInner.svelte'

	export let float: boolean = true
	export let hidden: boolean = false
	export let testJob: Job | undefined = undefined
	export let jobToWatch: { componentId: string; job: string } | undefined = undefined

	const { runnableJobEditorPanel, selectedComponentInEditor } =
		getContext<AppEditorContext>('AppEditorContext')
	let testIsLoading = false

	let testJobLoader: TestJobLoader

	$: $runnableJobEditorPanel.focused &&
		$selectedComponentInEditor &&
		$runnableJobEditorPanel.jobs &&
		updateSelectedJob()

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
</script>

<TestJobLoader bind:this={testJobLoader} bind:isLoading={testIsLoading} bind:job={testJob} />

{#if ($runnableJobEditorPanel.focused && $selectedComponentInEditor) || logDrawerOpen || resultDrawerOpen || !float}
	{@const frontendJob = $runnableJobEditorPanel?.frontendJobs[$selectedComponentInEditor ?? '']}
	<!-- svelte-ignore a11y-no-static-element-interactions -->
	{#if float}
		<div
			class="absolute z-[100] top-0 right-0 border-t h-full"
			style="width: {$runnableJobEditorPanel.width}px; transform: translateX({$runnableJobEditorPanel.width}px);"
		>
			<RunnableJobPanelInner {testIsLoading} {frontendJob} {testJob} />
		</div>
	{:else}
		<div class="flex flex-col w-full">
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
