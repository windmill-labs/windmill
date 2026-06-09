<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { base } from '$lib/base'
	import { goto } from '$app/navigation'
	import Button from '$lib/components/common/button/Button.svelte'
	import FolderPicker from '$lib/components/FolderPicker.svelte'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import { AssetService, type ListPipelineFoldersResponse } from '$lib/gen'
	import { resource } from 'runed'
	import { ArrowRight, Loader2 } from 'lucide-svelte'

	interface Props {
		open: boolean
		// When provided, the current folder is dropped from the existing list
		// so users don't "switch" to the folder they're already on.
		currentFolder?: string | undefined
	}
	let { open = $bindable(), currentFolder = undefined }: Props = $props()

	let pipelines = resource(
		() => $workspaceStore,
		async (ws) => {
			if (!ws) return [] as ListPipelineFoldersResponse
			return await AssetService.listPipelineFolders({ workspace: ws })
		}
	)

	let pickedFolder = $state('')

	let visiblePipelines = $derived(
		(pipelines.current ?? []).filter((p) => p.folder !== currentFolder)
	)

	async function openExistingPipeline(folder: string) {
		open = false
		await goto(`${base}/pipeline/${encodeURIComponent(folder)}`)
	}

	async function openPicked() {
		const name = pickedFolder.trim()
		if (!name) return
		await openExistingPipeline(name)
	}
</script>

<Modal bind:open title="Open a pipeline">
	<div class="flex flex-col gap-6 w-full min-w-0">
		{#if visiblePipelines.length > 0 || pipelines.loading}
			<section class="flex flex-col gap-2">
				<h3 class="text-xs font-semibold text-secondary uppercase tracking-wide">
					Existing pipelines
				</h3>
				{#if pipelines.loading && !pipelines.current}
					<div class="text-tertiary text-sm flex items-center gap-2">
						<Loader2 size={14} class="animate-spin" />
						Loading…
					</div>
				{:else if pipelines.error}
					<div class="text-red-500 text-sm">Failed: {pipelines.error.message}</div>
				{:else}
					<div class="flex flex-col border rounded-md overflow-hidden max-h-64 overflow-y-auto">
						{#each visiblePipelines as p}
							<button
								type="button"
								class="flex items-center justify-between px-3 py-2 border-b last:border-b-0 bg-surface hover:bg-surface-hover transition-colors text-left"
								onclick={() => openExistingPipeline(p.folder)}
							>
								<span class="font-mono text-sm">f/{p.folder}</span>
								<span class="text-2xs text-tertiary">
									{p.script_count}
									{p.script_count === 1 ? 'script' : 'scripts'}
								</span>
							</button>
						{/each}
					</div>
				{/if}
			</section>
		{/if}

		<section class="flex flex-col gap-2">
			<h3 class="text-xs font-semibold text-secondary uppercase tracking-wide">
				Pick or create a folder
			</h3>
			<div class="flex items-center gap-2">
				<div class="flex-1 min-w-0">
					<FolderPicker bind:folderName={pickedFolder} />
				</div>
				<Button
					variant="accent"
					unifiedSize="sm"
					disabled={!pickedFolder.trim()}
					onclick={openPicked}
					startIcon={{ icon: ArrowRight }}
				>
					Open
				</Button>
			</div>
		</section>
	</div>
</Modal>
