<script lang="ts">
	import { Button } from '$lib/components/common'
	import { ChevronDown, ChevronRight, History } from 'lucide-svelte'
	import type { AttachedTextFile } from './textFileUtils'

	let { content, files }: { content: string; files?: AttachedTextFile[] } = $props()

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
			<span class="inline-flex items-center gap-1 text-normal text-2xs">
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
			{#if files && files.length > 0}
				<!-- Attachments from the summarized turns, carried across the boundary
				     (still tool-readable) — named here, not rendered as interactive chips. -->
				<div class="mt-2 pt-2 border-t text-2xs text-tertiary">
					Files added by the user: {files.map((f) => f.name).join(', ')}
				</div>
			{/if}
		</div>
	{/if}
</div>
