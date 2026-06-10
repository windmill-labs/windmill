<script lang="ts">
	import { Folder, X, Loader2, AlertTriangle, Lock } from 'lucide-svelte'
	import type { AttachedFile } from './attachedFiles.svelte'

	let { folder, files, onRemove }: { folder: string; files: AttachedFile[]; onRemove: () => void } =
		$props()

	let showDelete = $state(false)

	// A not-yet-expanded folder (after reload) is a single placeholder entry.
	let locked = $derived(files.some((f) => f.status === 'locked'))
	let unavailable = $derived(files.some((f) => f.status === 'unavailable'))
	let indexing = $derived(files.some((f) => f.status === 'indexing'))
	let hasError = $derived(files.some((f) => f.status === 'error'))

	// Native tooltip listing the contained files (capped).
	let hoverList = $derived.by(() => {
		const max = 20
		const shown = files.slice(0, max).map((f) => f.name)
		if (files.length > max) shown.push(`… +${files.length - max} more`)
		return `${folder}/\n${shown.join('\n')}`
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
		aria-label={`Remove folder ${folder}`}
		onclick={(e) => {
			e.stopPropagation()
			onRemove()
		}}
	>
		{#if showDelete}
			<X size={16} />
		{:else if indexing}
			<Loader2 size={16} class="animate-spin text-tertiary" />
		{:else if hasError || unavailable}
			<AlertTriangle size={16} class="text-red-500" />
		{:else if locked}
			<Lock size={16} class="text-amber-500" />
		{:else}
			<Folder size={16} class="text-tertiary" />
		{/if}
	</button>
	<span class="truncate" title={folder}>{folder}</span>
</div>
