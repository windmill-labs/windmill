<script lang="ts">
	import { FileText, X, Loader2, AlertTriangle } from 'lucide-svelte'
	import type { AttachedFile } from './attachedFiles.svelte'

	let { file, onRemove }: { file: AttachedFile; onRemove: () => void } = $props()

	let showDelete = $state(false)

	function humanSize(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
		return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
	}

	let detail = $derived(
		file.status === 'indexing'
			? 'indexing…'
			: file.status === 'error'
				? 'failed'
				: `${file.lineCount} lines · ${humanSize(file.size)}`
	)
</script>

<div
	class="border rounded-md px-1 py-0.5 flex flex-row items-center gap-1 text-primary text-xs cursor-default hover:bg-surface-hover max-w-64 bg-surface"
	onmouseenter={() => (showDelete = true)}
	onmouseleave={() => (showDelete = false)}
	role="listitem"
	title={`${file.name} — ${detail}`}
>
	{#if file.status === 'indexing'}
		<Loader2 size={14} class="animate-spin shrink-0 text-tertiary" />
	{:else if file.status === 'error'}
		<AlertTriangle size={14} class="shrink-0 text-red-500" />
	{:else}
		<FileText size={14} class="shrink-0 text-tertiary" />
	{/if}
	<span class="truncate">{file.name}</span>
	<span class="text-tertiary shrink-0">{detail}</span>
	<button
		class="shrink-0 rounded-sm hover:bg-surface-hover-2 {showDelete ? 'opacity-100' : 'opacity-0'}"
		title="Remove file"
		aria-label={`Remove ${file.name}`}
		onclick={(e) => {
			e.stopPropagation()
			onRemove()
		}}
	>
		<X size={14} />
	</button>
</div>
