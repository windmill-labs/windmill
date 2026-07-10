<script lang="ts">
	import { ChevronDown, ChevronRight, Download, FileText, Trash2 } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import TimeAgo from '$lib/components/TimeAgo.svelte'
	import { getAiChatManager } from '../aiChatManagerContext'
	import { download, displayDate } from '$lib/utils'
	import { artifactFilename, artifactMimeType, type PersistedArtifact } from './artifactsDB'

	const aiChatManager = getAiChatManager()
	const artifacts = $derived(aiChatManager.artifacts.artifacts)

	let expanded = $state(true)

	function openArtifact(a: PersistedArtifact) {
		aiChatManager.openArtifact?.(a.id, a.name)
	}
</script>

{#if artifacts.length > 0}
	<div class="mb-1 border border-border-light rounded-md bg-surface overflow-hidden">
		<button
			class="w-full flex items-center gap-1.5 px-2 py-1 text-xs text-secondary hover:bg-surface-hover"
			onclick={() => (expanded = !expanded)}
		>
			{#if expanded}
				<ChevronDown size={14} class="shrink-0" />
			{:else}
				<ChevronRight size={14} class="shrink-0" />
			{/if}
			<FileText size={14} class="shrink-0" />
			<span class="font-medium">Artifacts ({artifacts.length})</span>
		</button>

		{#if expanded}
			<div role="list" class="flex flex-col border-t border-border-light max-h-24 overflow-y-auto">
				{#each artifacts as a (a.id)}
					<div
						class="flex items-center gap-1 pl-2 pr-1 py-1 hover:bg-surface-hover"
						role="listitem"
					>
						<button
							class="flex-1 min-w-0 text-left flex items-center gap-2"
							title={a.name}
							onclick={() => openArtifact(a)}
						>
							<span class="truncate text-xs font-normal text-secondary flex-1 min-w-0">{a.name}</span>
							<span
								class="shrink-0 text-2xs font-normal uppercase text-tertiary px-1 py-0.5 rounded bg-surface-secondary"
							>
								{a.kind}
							</span>
							<span
								class="shrink-0 text-2xs font-normal text-hint min-w-[4.5rem] text-right"
								title={displayDate(new Date(a.updatedAt))}
							>
								<TimeAgo date={new Date(a.updatedAt).toISOString()} noSeconds />
							</span>
						</button>
						<Button
							size="xs"
							variant="subtle"
							iconOnly
							title="Download"
							startIcon={{ icon: Download }}
							onClick={() => download(artifactFilename(a), a.content, artifactMimeType(a.kind))}
						/>
						<Button
							size="xs"
							color="red"
							variant="subtle"
							iconOnly
							title="Delete"
							startIcon={{ icon: Trash2 }}
							onClick={() => {
								aiChatManager.closeArtifact?.(a.id)
								void aiChatManager.artifacts.remove(a.id)
							}}
						/>
					</div>
				{/each}
			</div>
		{/if}
	</div>
{/if}
