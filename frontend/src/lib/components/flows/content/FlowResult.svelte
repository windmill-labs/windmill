<script lang="ts">
	import FlowPreviewResult from '$lib/components/FlowPreviewResult.svelte'
	import type { DurationStatus } from '$lib/components/graph'
	import type { Job } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import type { Writable } from 'svelte/store'

	interface Props {
		job?: Job
		isOwner?: boolean
		localDurationStatuses?: Writable<Record<string, DurationStatus>>
		suspendStatus?: Writable<Record<string, { job: Job; nb: number }>>
	}

	let { job, isOwner, localDurationStatuses, suspendStatus }: Props = $props()
</script>

{#if job && isOwner && localDurationStatuses && suspendStatus}
	<div class="p-4">
		<FlowPreviewResult
			{job}
			workspaceId={$workspaceStore}
			{isOwner}
			hideFlowResult={false}
			hideDownloadLogs={false}
			{localDurationStatuses}
			innerModules={[]}
			{suspendStatus}
		/>
	</div>
{:else}
	<p class="p-4 text-secondary">The result of the flow will be the result of the last node.</p>
{/if}
