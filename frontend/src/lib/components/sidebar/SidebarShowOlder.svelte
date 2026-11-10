<script module lang="ts">
	/** How recent a sidebar list's activity must be to show without expanding its older rows. */
	export const SIDEBAR_RECENT_MS = 7 * 24 * 60 * 60 * 1000
</script>

<script lang="ts">
	import { Ellipsis } from 'lucide-svelte'

	// The row under a sidebar list that reveals the rows inactive for over a week, and hides them
	// again.
	let {
		count,
		expanded,
		onToggle,
		rail = false
	}: { count: number; expanded: boolean; onToggle: () => void; rail?: boolean } = $props()

	const label = $derived(expanded ? 'Show less' : `Show ${count} older`)
</script>

<button
	type="button"
	onclick={onToggle}
	title={label}
	aria-expanded={expanded}
	class="flex flex-row items-center gap-2 px-2 h-6 shrink-0 w-full rounded text-xs font-normal text-tertiary hover:bg-surface-hover hover:text-secondary min-w-0"
>
	<Ellipsis size={14} class="shrink-0" />
	<span class={rail ? 'sr-only' : 'truncate'}>{label}</span>
</button>
