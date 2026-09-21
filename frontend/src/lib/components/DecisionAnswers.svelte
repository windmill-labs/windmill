<script lang="ts">
	import { JobService } from '$lib/gen'
	import { resource } from 'runed'
	import DisplayResult from './DisplayResult.svelte'

	interface Props {
		/** The job that asked the questions, which a branched AI decision's step no longer points at. */
		jobId: string
		workspaceId: string
	}

	let { jobId, workspaceId }: Props = $props()

	const answers = resource(
		() => ({ jobId, workspaceId }),
		({ jobId, workspaceId }) =>
			JobService.getCompletedJobResult({ workspace: workspaceId, id: jobId })
	)
</script>

<div>
	<div class="text-xs text-emphasis font-semibold mb-1">Answers</div>
	{#if answers.error}
		<p class="text-xs text-secondary">Could not load the answers</p>
	{:else}
		<div class="overflow-auto max-h-[200px]">
			<DisplayResult
				result={answers.current}
				loading={answers.loading}
				{jobId}
				{workspaceId}
				noControls
			/>
		</div>
	{/if}
</div>
