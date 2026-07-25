<script module lang="ts">
	// Shared trigger styling so every status-line token (Edits, Artifacts, Jobs)
	// stays visually uniform from one source.
	export const TOKEN_TRIGGER_CLASS =
		'inline-flex min-w-0 items-center gap-1.5 rounded px-1.5 py-0.5 text-xs text-secondary hover:bg-surface-hover'
</script>

<script lang="ts">
	import { ChevronUp } from 'lucide-svelte'

	// No <button>/onclick — the enclosing melt Popover owns the trigger and its click.
	let {
		label,
		expanded = false
	}: {
		label: string
		expanded?: boolean
	} = $props()
</script>

<span class="inline-flex min-w-0 items-center gap-1.5">
	<span class="min-w-0 truncate font-normal">
		{label}
	</span>
	<ChevronUp
		size={14}
		class={`shrink-0 transition-transform duration-150 ${expanded ? 'rotate-180' : ''}`}
	/>
</span>
