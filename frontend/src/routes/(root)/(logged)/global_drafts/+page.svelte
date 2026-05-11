<script lang="ts">
	import { Button } from '$lib/components/common'
	import {
		globalDraftStore,
		type WorkspaceItem
	} from '$lib/components/copilot/chat/global/draftStore.svelte'
	import { isGlobalAiEnabled } from '$lib/components/copilot/chat/global/gate'
	import { goto } from '$lib/navigation'
	import { Trash2 } from 'lucide-svelte'
	import { onMount } from 'svelte'

	onMount(() => {
		// Dev-only route. Bounce to home when the global mode gate is closed.
		if (!isGlobalAiEnabled()) {
			goto('/')
		}
	})

	let drafts = $derived(globalDraftStore.listDrafts())

	function draftKey(item: WorkspaceItem): string {
		return `${item.type}:${item.triggerKind ?? '-'}:${item.path}`
	}

	function deleteDraft(item: WorkspaceItem) {
		globalDraftStore.deleteDraft(item.type, item.path, item.triggerKind)
	}

	function clearAll() {
		globalDraftStore.clearDrafts()
	}
</script>

<div class="p-6 max-w-5xl mx-auto">
	<div class="flex items-center justify-between mb-6">
		<div>
			<h1 class="text-2xl font-semibold">Global AI drafts</h1>
			<p class="text-sm text-tertiary">
				Dev-only inspector for the in-memory global draft store.
			</p>
		</div>
		<Button
			variant="default"
			startIcon={{ icon: Trash2 }}
			disabled={drafts.length === 0}
			onclick={clearAll}
		>
			Clear all
		</Button>
	</div>

	{#if drafts.length === 0}
		<p class="text-sm text-tertiary">No drafts.</p>
	{:else}
		<ul class="space-y-4">
			{#each drafts as draft (draftKey(draft))}
				<li class="border rounded p-4">
					<div class="flex items-start justify-between gap-2 mb-2">
						<div>
							<div class="font-mono text-sm">
								<span class="font-semibold">{draft.type}</span>
								{#if draft.triggerKind}
									<span class="text-tertiary">({draft.triggerKind})</span>
								{/if}
								<span class="text-tertiary">·</span>
								<span>{draft.path}</span>
							</div>
							{#if draft.summary}
								<div class="text-sm text-tertiary mt-1">{draft.summary}</div>
							{/if}
							{#if draft.language}
								<div class="text-xs text-tertiary mt-1">language: {draft.language}</div>
							{/if}
						</div>
						<Button
							variant="default"
							startIcon={{ icon: Trash2 }}
							iconOnly
							onclick={() => deleteDraft(draft)}
						/>
					</div>
					<pre
						class="text-xs bg-surface-secondary p-3 rounded overflow-auto max-h-96 whitespace-pre-wrap">{JSON.stringify(
							draft.value,
							null,
							2
						)}</pre>
				</li>
			{/each}
		</ul>
	{/if}
</div>
