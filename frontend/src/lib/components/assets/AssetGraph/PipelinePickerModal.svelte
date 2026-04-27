<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { base } from '$lib/base'
	import { goto } from '$app/navigation'
	import Button from '$lib/components/common/button/Button.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import { FolderService, OpenAPI } from '$lib/gen'
	import { resource } from 'runed'
	import { sendUserToast } from '$lib/utils'
	import { ArrowRight, FolderPlus, Loader2 } from 'lucide-svelte'

	interface PipelineFolder {
		folder: string
		script_count: number
	}

	interface Props {
		open: boolean
		// When provided, the current folder is dropped from the existing list
		// so users don't "switch" to the folder they're already on.
		currentFolder?: string | undefined
	}
	let { open = $bindable(), currentFolder = undefined }: Props = $props()

	let pipelines = resource(
		() => $workspaceStore,
		async (ws, _prev, { signal }) => {
			if (!ws) return [] as PipelineFolder[]
			const base_url = OpenAPI.BASE ?? ''
			const res = await fetch(`${base_url}/w/${ws}/assets/pipelines`, {
				credentials: 'include',
				signal
			})
			if (!res.ok) throw new Error(`GET /assets/pipelines → ${res.status}`)
			return (await res.json()) as PipelineFolder[]
		}
	)

	let allFolders = resource(
		() => $workspaceStore,
		async (ws) => {
			if (!ws) return [] as string[]
			return await FolderService.listFolderNames({ workspace: ws })
		}
	)

	let selectedExistingFolder = $state<string | undefined>(undefined)
	let newFolderName = $state('')
	let creatingFolder = $state(false)

	let visiblePipelines = $derived(
		(pipelines.current ?? []).filter((p) => p.folder !== currentFolder)
	)

	let foldersWithoutPipeline = $derived.by(() => {
		const existing = new Set((pipelines.current ?? []).map((p) => p.folder))
		return (allFolders.current ?? []).filter((f) => !existing.has(f) && f !== currentFolder)
	})

	async function openExistingPipeline(folder: string) {
		open = false
		await goto(`${base}/pipeline/${encodeURIComponent(folder)}`)
	}

	async function startInExistingFolder() {
		if (!selectedExistingFolder) return
		await openExistingPipeline(selectedExistingFolder)
	}

	async function createFolderAndStart() {
		const name = newFolderName.trim()
		if (!name || !$workspaceStore) return
		creatingFolder = true
		try {
			await FolderService.createFolder({
				workspace: $workspaceStore,
				requestBody: { name }
			})
			sendUserToast(`Created folder f/${name}`)
			await openExistingPipeline(name)
		} catch (e: any) {
			sendUserToast(`Failed to create folder: ${e?.body ?? e?.message ?? e}`, true)
		} finally {
			creatingFolder = false
		}
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
									{p.script_count === 1 ? 'materializer' : 'materializers'}
								</span>
							</button>
						{/each}
					</div>
				{/if}
			</section>
		{/if}

		<section class="flex flex-col gap-2">
			<h3 class="text-xs font-semibold text-secondary uppercase tracking-wide">
				Start in an existing folder
			</h3>
			<div class="flex items-center gap-2">
				<div class="flex-1">
					<Select
						items={foldersWithoutPipeline.map((f) => ({ label: `f/${f}`, value: f }))}
						bind:value={selectedExistingFolder}
						placeholder={foldersWithoutPipeline.length === 0
							? 'No folders available'
							: 'Pick a folder…'}
						clearable
					/>
				</div>
				<Button
					variant="accent"
					unifiedSize="sm"
					disabled={!selectedExistingFolder}
					onclick={startInExistingFolder}
					startIcon={{ icon: ArrowRight }}
				>
					Open
				</Button>
			</div>
		</section>

		<section class="flex flex-col gap-2">
			<h3 class="text-xs font-semibold text-secondary uppercase tracking-wide">
				Or create a new folder
			</h3>
			<div class="flex items-center gap-2">
				<div class="flex-1">
					<TextInput
						bind:value={newFolderName}
						placeholder="new-folder-name"
						disabled={creatingFolder}
					/>
				</div>
				<Button
					variant="accent"
					unifiedSize="sm"
					disabled={!newFolderName.trim() || creatingFolder}
					onclick={createFolderAndStart}
					startIcon={{ icon: FolderPlus }}
				>
					{creatingFolder ? 'Creating…' : 'Create & open'}
				</Button>
			</div>
		</section>
	</div>
</Modal>
