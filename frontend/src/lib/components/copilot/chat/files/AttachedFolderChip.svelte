<script lang="ts">
	import { Folder, X, Loader2, AlertTriangle } from 'lucide-svelte'
	import type { AttachedFile } from './attachedFiles.svelte'

	let { folder, files, onRemove }: { folder: string; files: AttachedFile[]; onRemove: () => void } =
		$props()

	let showDelete = $state(false)

	function humanSize(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
		return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
	}

	let indexing = $derived(files.some((f) => f.status === 'indexing'))
	let hasError = $derived(files.some((f) => f.status === 'error'))
	let totalSize = $derived(files.reduce((acc, f) => acc + f.size, 0))
	let detail = $derived(
		indexing
			? 'indexing…'
			: `${files.length} file${files.length === 1 ? '' : 's'} · ${humanSize(totalSize)}`
	)

	// Native tooltip listing the contained files (capped).
	let hoverList = $derived.by(() => {
		const max = 20
		const shown = files.slice(0, max).map((f) => f.name)
		if (files.length > max) shown.push(`… +${files.length - max} more`)
		return `${folder}/\n${shown.join('\n')}`
	})
</script>

<div
	class="border rounded-md px-1 py-0.5 flex flex-row items-center gap-1 text-primary text-xs cursor-default hover:bg-surface-hover max-w-64 bg-surface"
	onmouseenter={() => (showDelete = true)}
	onmouseleave={() => (showDelete = false)}
	role="listitem"
	title={hoverList}
>
	{#if indexing}
		<Loader2 size={14} class="animate-spin shrink-0 text-tertiary" />
	{:else if hasError}
		<AlertTriangle size={14} class="shrink-0 text-red-500" />
	{:else}
		<Folder size={14} class="shrink-0 text-tertiary" />
	{/if}
	<span class="truncate">{folder}</span>
	<span class="text-tertiary shrink-0">{detail}</span>
	<button
		class="shrink-0 rounded-sm hover:bg-surface-hover-2 {showDelete ? 'opacity-100' : 'opacity-0'}"
		title="Remove folder"
		aria-label={`Remove folder ${folder}`}
		onclick={(e) => {
			e.stopPropagation()
			onRemove()
		}}
	>
		<X size={14} />
	</button>
</div>
