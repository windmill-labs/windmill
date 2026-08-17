<script lang="ts">
	import { userStore, workspaceStore } from '$lib/stores'
	import { base } from '$lib/base'
	import { goto } from '$app/navigation'
	import Button from '$lib/components/common/button/Button.svelte'
	import FolderPicker from '$lib/components/FolderPicker.svelte'
	import { AssetService, type ListPipelineFoldersResponse } from '$lib/gen'
	import { resource } from 'runed'
	import { ArrowRight, Loader2 } from 'lucide-svelte'

	interface Props {
		// When provided, the current folder is dropped from the existing list
		// so users don't "switch" to the folder they're already on.
		currentFolder?: string | undefined
		// Carried over to the target URL so switching folders from the
		// editor keeps the user in edit mode; the default view mode needs
		// no param.
		mode?: 'view' | 'edit'
		// Called right before navigating — lets a wrapping modal close itself.
		onOpen?: () => void
	}
	let { currentFolder = undefined, mode = 'view', onOpen = undefined }: Props = $props()

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
		onOpen?.()
		const modeQuery = mode !== 'view' ? `?mode=${mode}` : ''
		await goto(`${base}/pipeline/${encodeURIComponent(folder)}${modeQuery}`)
	}

	async function openPicked() {
		const name = pickedFolder.trim()
		if (!name) return
		await openExistingPipeline(name)
	}
</script>

<div class="flex flex-col gap-6 w-full min-w-0">
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
		{:else if visiblePipelines.length === 0}
			<div class="text-tertiary text-sm">
				{currentFolder
					? 'No other pipelines in this workspace.'
					: 'No pipelines yet. A pipeline is any folder whose scripts carry pipeline annotations.'}
			</div>
		{:else}
			<div class="flex flex-col border rounded-md overflow-hidden max-h-64 overflow-y-auto">
				{#each visiblePipelines as p (p.folder)}
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

	{#if !$userStore?.operator}
		<!-- Operators only view existing pipelines — opening an arbitrary
		     folder is a build-a-new-pipeline affordance. -->
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
	{/if}
</div>
