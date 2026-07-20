<script lang="ts">
	import { Button } from '$lib/components/common'
	import { ChevronDown, ChevronRight, History } from 'lucide-svelte'
	import ContextElementBadge from './ContextElementBadge.svelte'
	import { createAttachedFileContextElement } from './context'
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
			<span class="inline-flex items-center gap-1 text-2xs text-tertiary">
				<History size={12} />
				Summarized earlier conversation
			</span>
		</Button>
		<div class="h-px flex-1 bg-surface-selected"></div>
	</div>
	{#if files && files.length > 0}
		<!-- Attachments from the summarized turns, carried across the boundary —
		     they stay readable, so keep them visible where their messages were. -->
		<div class="mt-1.5 flex flex-row flex-wrap items-center gap-1 justify-center">
			<!-- Index in the key: same-named entries can survive in older transcripts. -->
			{#each files as file, i (`${file.name}:${i}`)}
				<ContextElementBadge
					contextElement={createAttachedFileContextElement(file.name, file.content)}
					compact
				/>
			{/each}
		</div>
	{/if}
	{#if expanded}
		<div
			class="mt-2 max-h-80 overflow-y-auto whitespace-pre-wrap rounded-md bg-surface-secondary p-3 text-xs text-secondary"
		>
			{content}
		</div>
	{/if}
</div>
