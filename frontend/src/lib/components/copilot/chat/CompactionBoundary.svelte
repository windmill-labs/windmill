<script lang="ts">
	import { Button } from '$lib/components/common'
	import { ChevronDown, ChevronRight, History } from 'lucide-svelte'

	let { content }: { content: string } = $props()

	let expanded = $state(false)
</script>

<div class="my-4 px-2">
	<div class="flex items-center gap-2">
		<div class="h-px flex-1 bg-surface-selected"></div>
		<Button
			variant="subtle"
			size="xs2"
			startIcon={{ icon: expanded ? ChevronDown : ChevronRight }}
			onclick={() => (expanded = !expanded)}
		>
			<span class="inline-flex items-center gap-1 text-2xs text-tertiary">
				<History size={12} />
				Summarized earlier conversation
			</span>
		</Button>
		<div class="h-px flex-1 bg-surface-selected"></div>
	</div>
	{#if expanded}
		<div
			class="mt-2 max-h-80 overflow-y-auto whitespace-pre-wrap rounded-md bg-surface-secondary p-3 text-xs text-secondary"
		>
			{content}
		</div>
	{/if}
</div>
