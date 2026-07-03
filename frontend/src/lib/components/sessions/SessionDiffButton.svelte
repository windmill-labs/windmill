<script lang="ts">
	import { Button } from '$lib/components/common'
	import { DiffIcon } from 'lucide-svelte'

	// Shared diff-drawer trigger used by both the fork bar and the draft bar: a
	// default button showing the ± diff icon and the item count.
	let {
		count,
		onclick,
		title = 'Open diff',
		disabled = false
	}: { count: number; onclick: () => void; title?: string; disabled?: boolean } = $props()
</script>

<Button
	variant="default"
	unifiedSize="xs"
	startIcon={{ icon: DiffIcon }}
	{disabled}
	{title}
	{onclick}
>
	{count}
</Button>
