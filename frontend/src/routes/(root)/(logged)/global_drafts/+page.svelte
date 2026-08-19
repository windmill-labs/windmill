<script lang="ts">
	import { Button } from '$lib/components/common'
	import {
		clearGlobalDrafts,
		deleteGlobalDraft,
		listGlobalDrafts
	} from '$lib/components/copilot/chat/global/userDraftAdapter'
	import type { WorkspaceItem } from '$lib/components/copilot/chat/global/workspaceItems'
	import { goto } from '$lib/navigation'
	import { workspaceStore } from '$lib/stores'
	import { Trash2 } from 'lucide-svelte'
	import { onMount } from 'svelte'
	import { resource } from 'runed'

	let enabled = $state(false)
	let refreshToken = $state(0)

	function refreshDrafts() {
		refreshToken += 1
	}

	onMount(() => {
		// Dev tooling, not part of the sessions beta — only reachable on dev builds.
		enabled = import.meta.env.DEV
		if (!enabled) {
			goto('/')
		}

		const onStorage = (event: StorageEvent) => {
			if (event.key?.startsWith('userdraft/')) refreshDrafts()
		}
		window.addEventListener('storage', onStorage)
		// Same-tab saves and live editor registry changes don't emit `storage`.
		const interval = window.setInterval(refreshDrafts, 1000)

		return () => {
			window.removeEventListener('storage', onStorage)
			window.clearInterval(interval)
		}
	})

	const draftsResource = resource(
		() => ({ ws: $workspaceStore, token: refreshToken }),
		async ({ ws }) => (ws ? await listGlobalDrafts(ws) : [])
	)
	let drafts = $derived(draftsResource.current ?? [])

	function draftKey(item: WorkspaceItem): string {
		return `${item.type}:${item.triggerKind ?? '-'}:${item.path}`
	}

	async function deleteDraft(item: WorkspaceItem) {
		if (!$workspaceStore) return
		await deleteGlobalDraft($workspaceStore, item.type, item.path, item.triggerKind)
		refreshDrafts()
	}

	async function clearAll() {
		if (!$workspaceStore) return
		// Delete each listed draft from the backend (the source of truth) — the
		// local clearGlobalDrafts only clears in-tab cells, leaving persisted rows.
		// Continue past a per-row failure so one bad delete doesn't strand the rest.
		for (const item of [...drafts]) {
			try {
				await deleteGlobalDraft($workspaceStore, item.type, item.path, item.triggerKind)
			} catch (e) {
				console.error('Failed to clear draft', item.path, e)
			}
		}
		clearGlobalDrafts($workspaceStore)
		refreshDrafts()
	}
</script>

{#if enabled}
	<div class="p-6 max-w-5xl mx-auto">
		<div class="flex items-center justify-between mb-6">
			<div>
				<h1 class="text-2xl font-semibold">Global local drafts</h1>
				<p class="text-sm text-tertiary"> Dev-only inspector for global local drafts. </p>
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
							class="text-xs bg-surface-secondary p-3 rounded overflow-auto max-h-96 whitespace-pre-wrap"
							>{JSON.stringify(draft.value, null, 2)}</pre
						>
					</li>
				{/each}
			</ul>
		{/if}
	</div>
{/if}
