<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '../types'
	import TestJobLoader from '$lib/components/TestJobLoader.svelte'
	import type { Job } from '$lib/gen'
	import RunnalbeJobPanelInner from './RunnalbeJobPanelInner.svelte'

	export let float: boolean = true

	const { runnableJobEditorPanel, selectedComponentInEditor } =
		getContext<AppEditorContext>('AppEditorContext')
	let testJob: Job | undefined = undefined
	let testIsLoading = false

	let testJobLoader: TestJobLoader

	let jobToWatch: { componentId: string; job: string } | undefined = undefined
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

{#if ($runnableJobEditorPanel.focused && $selectedComponentInEditor) || logDrawerOpen || resultDrawerOpen}
	{@const frontendJob = $runnableJobEditorPanel?.frontendJobs[$selectedComponentInEditor ?? '']}
	<!-- svelte-ignore a11y-no-static-element-interactions -->
	{#if float}
		<div
			class="absolute z-[100] top-0 right-0 border-t h-full"
			style="width: {$runnableJobEditorPanel.width}px; transform: translateX({$runnableJobEditorPanel.width}px);"
		>
			<RunnalbeJobPanelInner {frontendJob} {testJob} />
		</div>
	{:else}
		<div class="flex flex-col w-full">
			<RunnalbeJobPanelInner {frontendJob} {testJob} />
		</div>
	{/if}
{/if}
