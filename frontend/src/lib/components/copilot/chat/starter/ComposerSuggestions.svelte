<script lang="ts">
	import { Pencil, Play } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import type { ComposerSuggestion } from './starterPrompts.svelte'

	let {
		suggestions,
		onPick
	}: {
		suggestions: ComposerSuggestion[]
		onPick: (prompt: string) => void
	} = $props()
</script>

<!-- One row whatever the width: it scrolls sideways and fades out at its end, so the page under
     the composer does not move with the window width. -->
<div
	class="flex flex-row items-center gap-1.5 overflow-x-auto no-scrollbar [mask-image:linear-gradient(90deg,#000_calc(100%-24px),transparent)]"
>
	{#each suggestions.slice(0, 4) as suggestion (suggestion.label)}
		<Button
			variant="default"
			unifiedSize="sm"
			btnClasses="!rounded-full !text-xs !text-secondary !font-normal shrink-0"
			title={suggestion.mode === 'use' ? 'Use: look, ask or run' : 'Build: create or change'}
			startIcon={{ icon: suggestion.mode === 'use' ? Play : Pencil, classes: 'text-hint' }}
			onClick={() => onPick(suggestion.prompt)}
		>
			{suggestion.label}
		</Button>
	{/each}
</div>
