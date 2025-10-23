<script lang="ts">
	import FlowPreviewResult from '$lib/components/FlowPreviewResult.svelte'
	import type { Job } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import FlowCard from '../common/FlowCard.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import type { StateStore } from '$lib/utils'

	interface Props {
		job?: Job
		isOwner?: boolean
		suspendStatus?: StateStore<Record<string, { job: Job; nb: number }>>
		noEditor: boolean
		onOpenDetails?: () => void
	}

	let { job, isOwner, suspendStatus, noEditor, onOpenDetails }: Props = $props()
</script>

<FlowCard {noEditor} title="Flow result">
	{#if job && isOwner !== undefined && suspendStatus}
		<div class="px-4 py-2">
			<FlowPreviewResult
				{job}
				workspaceId={$workspaceStore}
				{isOwner}
				hideFlowResult={false}
				hideDownloadLogs={false}
				innerModules={[]}
				{suspendStatus}
				{extra}
				hideJobId
			/>
		</div>
	{:else}
		<p class="p-4 text-secondary text-xs">
			The result of the flow will be the result of the last node.
		</p>
	{/if}
</FlowCard>

{#snippet extra()}
	<div class="flex justify-end">
		<Button variant="default" size="xs" on:click={() => onOpenDetails?.()}>Open details</Button>
	</div>
{/snippet}
