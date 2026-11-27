<script lang="ts">
	import { Folder, X, Loader2, AlertTriangle, Lock } from 'lucide-svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { Button } from '$lib/components/common'
	import { getFileIcon } from '$lib/components/icons/fileIcon'
	import type { AttachedFile, AttachedFolder } from './attachedFiles.svelte'
	import { isViewableImageName } from './imageFiles'

	let {
		folder,
		onRemove,
		onOpenImage
	}: {
		folder: AttachedFolder
		onRemove: () => void
		/** Shows an image of the folder beside the chat; without it images are listed only. */
		onOpenImage?: (file: AttachedFile) => void
	} = $props()

	let showDelete = $state(false)
	let popover: Popover | undefined = $state(undefined)

	// The list renders every row; past this a folder is a build output, not something to browse.
	const MAX_LISTED = 300

	const sorted = $derived(
		[...folder.files].sort((a, b) => (a.relPath ?? a.name).localeCompare(b.relPath ?? b.name))
	)
	const imageCount = $derived(folder.files.filter((f) => isOpenable(f)).length)

	function isOpenable(f: AttachedFile): boolean {
		return f.status === 'ready' && isViewableImageName(f.sourceName ?? f.name)
	}

	/** Path inside the folder: the folder's own name is the chip's label already. */
	function innerPath(f: AttachedFile): string {
		const path = f.relPath ?? f.name
		const prefix = `${folder.name}/`
		return path.startsWith(prefix) ? path.slice(prefix.length) : path
	}

	const canList = $derived(folder.status !== 'locked' && folder.status !== 'unavailable')
</script>

<div
	class="border rounded-md px-1 py-0.5 flex flex-row items-center gap-1 text-primary text-xs font-normal cursor-default hover:bg-surface-hover max-w-48 bg-surface"
	onmouseenter={() => (showDelete = true)}
	onmouseleave={() => (showDelete = false)}
	role="listitem"
>
	<button
		class="shrink-0"
		aria-label={`Remove folder ${folder.name}`}
		onclick={(e) => {
			e.stopPropagation()
			onRemove()
		}}
	>
		{#if showDelete}
			<X size={16} />
		{:else if folder.status === 'indexing'}
			<Loader2 size={16} class="animate-spin text-tertiary" />
		{:else if folder.status === 'error' || folder.status === 'unavailable'}
			<AlertTriangle size={16} class="text-red-500" />
		{:else if folder.status === 'locked'}
			<Lock size={16} class="text-amber-500" />
		{:else}
			<Folder size={16} class="text-tertiary" />
		{/if}
	</button>
	{#if canList}
		<Popover bind:this={popover} placement="top-start" class="min-w-0" contentClasses="p-0">
			{#snippet trigger()}
				<span class="block truncate cursor-pointer" title={`Show the files in ${folder.name}`}
					>{folder.name}</span
				>
			{/snippet}
			{#snippet content()}
				<div class="flex flex-col w-72 max-w-[80vw]">
					<div class="px-3 py-2 border-b border-border-light text-2xs text-secondary">
						<span class="font-semibold text-emphasis">{folder.name}/</span>
						· {folder.files.length}
						{folder.files.length === 1 ? 'file' : 'files'}{imageCount > 0
							? ` · ${imageCount} ${imageCount === 1 ? 'image' : 'images'}`
							: ''}
					</div>
					<ul class="max-h-72 overflow-y-auto py-1" aria-label={`Files in ${folder.name}`}>
						{#each sorted.slice(0, MAX_LISTED) as f (f.name)}
							{@const icon = getFileIcon(f.name)}
							{@const Icon = icon.icon}
							<li>
								{#if onOpenImage && isOpenable(f)}
									<Button
										variant="subtle"
										unifiedSize="sm"
										wrapperClasses="w-full"
										btnClasses="w-full !justify-start rounded-none font-normal"
										title={`Open ${innerPath(f)} in the preview`}
										onClick={() => {
											onOpenImage(f)
											popover?.close()
										}}
									>
										<Icon size={14} class="shrink-0 {icon.className ?? ''}" />
										<span class="truncate">{innerPath(f)}</span>
									</Button>
								{:else}
									<div
										class="flex items-center gap-2 px-3 py-1 text-xs text-secondary"
										title={innerPath(f)}
									>
										<Icon size={14} class="shrink-0 {icon.className ?? ''}" />
										<span class="truncate">{innerPath(f)}</span>
									</div>
								{/if}
							</li>
						{:else}
							<li class="px-3 py-1 text-xs text-tertiary">No files linked from this folder.</li>
						{/each}
						{#if sorted.length > MAX_LISTED}
							<li class="px-3 py-1 text-2xs text-tertiary">
								… {sorted.length - MAX_LISTED} more
							</li>
						{/if}
					</ul>
				</div>
			{/snippet}
		</Popover>
	{:else}
		<span
			class="truncate"
			title={folder.status === 'locked'
				? `${folder.name}/ — locked, click send to restore`
				: `${folder.name}/ — unavailable`}>{folder.name}</span
		>
	{/if}
</div>
