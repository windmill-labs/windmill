<script lang="ts">
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import LogViewer from '$lib/components/LogViewer.svelte'
	import { Splitpanes, Pane } from 'svelte-splitpanes'
	import type { Job } from '$lib/gen'
	import { Loader2 } from 'lucide-svelte'

	export let frontendJob: boolean | any = false
	export let testJob: Job | any = undefined
	export let testIsLoading = false

	let logDrawerOpen = false
	let resultDrawerOpen = false
</script>

<Splitpanes horizontal>
	<Pane size={frontendJob ? 30 : 50} minSize={10}>
		{#if frontendJob}
			<div class="p-2 bg-surface-secondary h-full w-full">
				<div class="text-sm text-tertiary pb-4">Frontend Job</div>
				<div class="text-2xs text-tertiary">Check your browser console to see the logs</div>
			</div>
		{:else}
			<LogViewer
				bind:drawerOpen={logDrawerOpen}
				small
				jobId={testJob?.id}
				duration={testJob?.['duration_ms']}
				mem={testJob?.['mem_peak']}
				content={testJob?.logs}
				isLoading={testIsLoading && testJob?.['running'] == false}
				tag={testJob?.tag}
			/>
		{/if}
	</Pane>
	<Pane size={frontendJob ? 70 : 50} minSize={10} class="text-sm text-tertiary">
		{#if frontendJob}
			<div class="break-words relative h-full px-1">
				<DisplayResult bind:drawerOpen={resultDrawerOpen} result={frontendJob} />
			</div>
		{:else if testJob != undefined && 'result' in testJob && testJob.result != undefined}
			<div class="break-words relative h-full px-1">
				<DisplayResult
					bind:drawerOpen={resultDrawerOpen}
					workspaceId={testJob?.workspace_id}
					jobId={testJob?.id}
					result={testJob.result}
					language={testJob?.language}
				/></div
			>
		{:else}
			<div class="px-1 pt-1">
				{#if testIsLoading}
					<Loader2 class="animate-spin" />
				{:else}
					Test to see the result here
				{/if}
			</div>
		{/if}
	</Pane>
</Splitpanes>
