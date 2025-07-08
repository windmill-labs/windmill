<script lang="ts">
	import FlowPreviewResult from '$lib/components/FlowPreviewResult.svelte'
	import type { DurationStatus } from '$lib/components/graph'
	import type { Job } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import type { Writable } from 'svelte/store'
	import FlowCard from '../common/FlowCard.svelte'
	import Button from '$lib/components/common/button/Button.svelte'

	interface Props {
		job?: Job
		isOwner?: boolean
		localDurationStatuses?: Writable<Record<string, DurationStatus>>
		suspendStatus?: Writable<Record<string, { job: Job; nb: number }>>
		noEditor: boolean
		onOpenDetails?: () => void
	}

	let { job, isOwner, localDurationStatuses, suspendStatus, noEditor, onOpenDetails }: Props =
		$props()
</script>

<FlowCard {noEditor} title="Flow result">
	{#if job && isOwner && localDurationStatuses && suspendStatus}
		<div class="px-4 py-2">
			<FlowPreviewResult
				{job}
				workspaceId={$workspaceStore}
				{isOwner}
				hideFlowResult={false}
				hideDownloadLogs={false}
				{localDurationStatuses}
				innerModules={[]}
				{suspendStatus}
				{extra}
				hideJobId
			/>
		</div>
	{:else}
		<p class="p-4 text-secondary">The result of the flow will be the result of the last node.</p>
	{/if}
</FlowCard>

{#snippet extra()}
	<div class="flex justify-end">
		<Button variant="border" color="light" size="xs" on:click={() => onOpenDetails?.()}
			>Open details</Button
		>
	</div>
{/snippet}
