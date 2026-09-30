<script lang="ts">
	import { userStore, workspaceStore } from '$lib/stores'
	import PipelineFolderList from '$lib/components/assets/AssetGraph/PipelineFolderList.svelte'
	import PipelineSetupSignpost from '$lib/components/assets/AssetGraph/PipelineSetupSignpost.svelte'
	import PipelineAlphaAckModal from '$lib/components/assets/AssetGraph/PipelineAlphaAckModal.svelte'
	import { BookOpen, NetworkIcon } from 'lucide-svelte'
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

<div class="flex flex-col h-full">
	<div
		class="border-b flex flex-row justify-between gap-2 px-2 py-1 items-center min-h-10 shrink-0 whitespace-nowrap"
	>
		<div class="flex flex-row items-center gap-2">
			<NetworkIcon size={16} class="text-secondary shrink-0" />
			<h1 class="text-sm font-semibold text-emphasis">Pipelines</h1>
			<Badge color="green">Alpha</Badge>
		</div>
	</div>

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
