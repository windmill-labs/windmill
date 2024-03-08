<script lang="ts">
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import LogViewer from '$lib/components/LogViewer.svelte'
	import { getContext } from 'svelte'
	import { Splitpanes, Pane } from 'svelte-splitpanes'
	import type { AppEditorContext } from '../types'
	import TestJobLoader from '$lib/components/TestJobLoader.svelte'
	import type { Job } from '$lib/gen'
	import { Loader2 } from 'lucide-svelte'

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
</script>

<TestJobLoader bind:this={testJobLoader} bind:isLoading={testIsLoading} bind:job={testJob} />

{#if $runnableJobEditorPanel.focused && $selectedComponentInEditor}
	{@const frontendJob = $runnableJobEditorPanel?.frontendJobs[$selectedComponentInEditor]}
	<!-- svelte-ignore a11y-no-static-element-interactions -->
	<div
		class="absolute z-[100] top-0 right-0 border-t h-full"
		style="width: {$runnableJobEditorPanel.width}px; transform: translateX({$runnableJobEditorPanel.width}px);"
	>
		<Splitpanes horizontal>
			<Pane size={frontendJob ? 30 : 50} minSize={10}>
				{#if frontendJob}
					<div class="p-2 bg-surface-secondary h-full w-full">
						<div class="text-sm text-tertiary pb-4">Frontend Job</div>
						<div class="text-2xs text-tertiary">Check your browser console to see the logs</div>
					</div>
				{:else}
					<LogViewer
						small
						jobId={testJob?.id}
						duration={testJob?.['duration_ms']}
						mem={testJob?.['mem_peak']}
						content={testJob?.logs}
						isLoading={testIsLoading}
						tag={testJob?.tag}
					/>
				{/if}
			</Pane>
			<Pane size={frontendJob ? 70 : 50} minSize={10} class="text-sm text-tertiary">
				{#if frontendJob}
					<pre class="overflow-x-auto break-words relative h-full px-2"
						><DisplayResult result={frontendJob}>
                    
                                                        <!-- <svelte:fragment slot="copilot-fix">
                                                            {#if lang && editor && diffEditor && stepArgs && testJob?.result?.error}
                                                                    <ScriptFix
                                                                        error={JSON.stringify(testJob.result.error)}
                                                                        {lang}
                                                                        {editor}
                                                                        {diffEditor}
                                                                        args={stepArgs}
                                                                    />
                                                                {/if}
                                                        </svelte:fragment> -->
                                                    </DisplayResult>
                                                </pre>
				{:else if testJob != undefined && 'result' in testJob && testJob.result != undefined}
					<pre class="overflow-x-auto break-words relative h-full px-2"
						><DisplayResult
							workspaceId={testJob?.workspace_id}
							jobId={testJob?.id}
							result={testJob.result}>
                            
																<!-- <svelte:fragment slot="copilot-fix">
																	{#if lang && editor && diffEditor && stepArgs && testJob?.result?.error}
																			<ScriptFix
																				error={JSON.stringify(testJob.result.error)}
																				{lang}
																				{editor}
																				{diffEditor}
																				args={stepArgs}
																			/>
																		{/if}
																</svelte:fragment> -->
															</DisplayResult>
														</pre>
				{:else}
					<div class="p-2">
						{#if testIsLoading}
							<Loader2 class="animate-spin" />
						{:else}
							Test to see the result here
						{/if}
					</div>
				{/if}
			</Pane>
		</Splitpanes>
	</div>
{/if}
