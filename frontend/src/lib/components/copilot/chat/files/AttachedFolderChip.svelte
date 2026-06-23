<script lang="ts">
	import { Folder, X, Loader2, AlertTriangle, Lock } from 'lucide-svelte'
	import type { AttachedFolder } from './attachedFiles.svelte'

	let { folder, onRemove }: { folder: AttachedFolder; onRemove: () => void } = $props()

	let showDelete = $state(false)

	// Native tooltip listing the contained files (capped).
	let hoverList = $derived.by(() => {
		if (folder.status === 'locked') return `${folder.name}/ — locked, click send to restore`
		if (folder.status === 'unavailable') return `${folder.name}/ — unavailable`
		const max = 20
		const shown = folder.files.slice(0, max).map((f) => f.name)
		if (folder.files.length > max) shown.push(`… +${folder.files.length - max} more`)
		return `${folder.name}/\n${shown.join('\n')}`
	})
</script>

<div
	class="border rounded-md px-1 py-0.5 flex flex-row items-center gap-1 text-primary text-xs font-normal cursor-default hover:bg-surface-hover hover:cursor-pointer max-w-48 bg-surface"
	onmouseenter={() => (showDelete = true)}
	onmouseleave={() => (showDelete = false)}
	role="listitem"
	title={hoverList}
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
	<span class="truncate" title={folder.name}>{folder.name}</span>
</div>
