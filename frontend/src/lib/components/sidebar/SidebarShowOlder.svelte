<script module lang="ts">
	/** How recent a sidebar list's activity must be to show without expanding its older rows. */
	export const SIDEBAR_RECENT_MS = 7 * 24 * 60 * 60 * 1000
</script>

<script lang="ts">
	import { Ellipsis } from 'lucide-svelte'
	import { Button } from '$lib/components/common'

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

<!-- mx-0.5 gives the 12px icon a 16px item icon's slot: it sits on the rail's icon column and the
     label starts where the other rows' labels do. -->
<Button
	unifiedSize="xs"
	variant="subtle"
	startIcon={{ icon: Ellipsis, classes: 'mx-0.5' }}
	iconOnly={rail}
	title={label}
	aria-label={label}
	aria-expanded={expanded}
	onClick={onToggle}
	btnClasses="justify-start px-2 font-normal text-tertiary hover:text-secondary"
>
	{label}
</Button>
