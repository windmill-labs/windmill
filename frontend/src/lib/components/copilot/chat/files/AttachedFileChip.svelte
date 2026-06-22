<script lang="ts">
	import { X, Loader2, AlertTriangle, Lock } from 'lucide-svelte'
	import type { AttachedFile } from './attachedFiles.svelte'
	import { getFileIcon } from '$lib/components/icons/fileIcon'

	let { file, onRemove }: { file: AttachedFile; onRemove: () => void } = $props()

	let showDelete = $state(false)
	let fileIcon = $derived(getFileIcon(file.name))

	function humanSize(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
		return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
	}

	// Metadata/status lives in the hover tooltip — the chip itself stays
	// label-only to match the context-element chips it sits beside.
	let detail = $derived(
		file.status === 'indexing'
			? 'indexing…'
			: file.status === 'error'
				? 'failed'
				: file.status === 'locked'
					? 'locked — click send to restore'
					: file.status === 'unavailable'
						? 'unavailable'
						: `${file.lineCount} lines · ${humanSize(file.size)}`
	)
</script>

<div
	class="border rounded-md px-1 py-0.5 flex flex-row items-center gap-1 text-primary text-xs font-normal cursor-default hover:bg-surface-hover hover:cursor-pointer max-w-48 bg-surface"
	onmouseenter={() => (showDelete = true)}
	onmouseleave={() => (showDelete = false)}
	role="listitem"
	title={`${file.name} — ${detail}`}
>
	<button
		class="shrink-0"
		aria-label={`Remove ${file.name}`}
		onclick={(e) => {
			e.stopPropagation()
			onRemove()
		}}
	>
		{#if showDelete}
			<X size={16} />
		{:else if file.status === 'indexing'}
			<Loader2 size={16} class="animate-spin text-tertiary" />
		{:else if file.status === 'error' || file.status === 'unavailable'}
			<AlertTriangle size={16} class="text-red-500" />
		{:else if file.status === 'locked'}
			<Lock size={16} class="text-amber-500" />
		{:else}
			{@const Icon = fileIcon.icon}
			<Icon size={16} class={fileIcon.className ?? ''} />
		{/if}
	</button>
	<span class="truncate" title={file.name}>{file.name}</span>
</div>
