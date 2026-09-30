<script lang="ts">
	import { userStore, workspaceStore } from '$lib/stores'
	import PipelineFolderList from '$lib/components/assets/AssetGraph/PipelineFolderList.svelte'
	import PipelineSetupSignpost from '$lib/components/assets/AssetGraph/PipelineSetupSignpost.svelte'
	import PipelineAlphaAckModal from '$lib/components/assets/AssetGraph/PipelineAlphaAckModal.svelte'
	import { BookOpen } from 'lucide-svelte'
	import PageHeaderContent from '$lib/components/PageHeaderContent.svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import { onMount } from 'svelte'

	const ACK_STORAGE_KEY = 'pipeline-alpha-ack'

	// Gate the index behind a one-time alpha acknowledgement. We read
	// localStorage in onMount (not at module scope) so SSR doesn't blow up.
	let ackOpen = $state(false)

	onMount(() => {
		const acked =
			typeof localStorage !== 'undefined' && localStorage.getItem(ACK_STORAGE_KEY) === 'true'
		if (!acked) {
			ackOpen = true
		}
	})

	function handleAck() {
		try {
			localStorage.setItem(ACK_STORAGE_KEY, 'true')
		} catch {
			// Storage may be unavailable (private mode, quota); the ack still
			// flows through for this visit, the user just sees the modal again next time.
		}
		ackOpen = false
	}
</script>

<svelte:head>
	<title>Pipelines — Windmill</title>
</svelte:head>

<PageHeaderContent section={{ label: 'Pipelines' }} afterName={alphaBadge} />

{#snippet alphaBadge()}
	<Badge color="green">Alpha</Badge>
{/snippet}

<div class="flex flex-col h-full">

	<div class="flex-1 min-h-0 overflow-y-auto bg-surface">
		<div class="max-w-2xl mx-auto flex flex-col gap-6 px-4 py-8">
			<div class="flex flex-col gap-2">
				<h2 class="text-lg font-semibold text-emphasis">Data pipelines</h2>
				<p class="text-xs text-secondary">
					Chain ingestion, transformation and materialization steps into an asset-aware graph.
				</p>
				<a
					href="https://www.windmill.dev/docs/pipelines"
					target="_blank"
					rel="noreferrer"
					class="text-xs text-accent hover:underline inline-flex items-center gap-1 w-fit"
				>
					<BookOpen size={12} />
					Pipelines documentation
				</a>
			</div>

			{#if !$userStore?.operator && $workspaceStore}
				<PipelineSetupSignpost workspace={$workspaceStore} />
			{/if}

			<PipelineFolderList hideExisting />
		</div>
	</div>
</div>

<PipelineAlphaAckModal bind:open={ackOpen} onAck={handleAck} />
