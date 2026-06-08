<script lang="ts">
	import { Button } from '$lib/components/common'
	import { DraftService, type ListDraftsResponse } from '$lib/gen'
	import { isGlobalAiEnabled } from '$lib/components/copilot/chat/global/gate'
	import { goto } from '$lib/navigation'
	import { workspaceStore } from '$lib/stores'
	import { Trash2 } from 'lucide-svelte'
	import { onMount } from 'svelte'

	type DraftRow = ListDraftsResponse[number]

	let enabled = $state(false)
	let drafts = $state<DraftRow[]>([])

	async function loadDrafts() {
		drafts = $workspaceStore ? await DraftService.listDrafts({ workspace: $workspaceStore }) : []
	}

	// `value: null` is the canonical delete; `force` skips the conflict check
	// (this is a debug tool, the user just wants the row gone).
	async function removeDraft(ws: string, row: DraftRow): Promise<void> {
		await DraftService.saveDraft({
			workspace: ws,
			kind: row.typ,
			path: row.path,
			requestBody: { value: null, force: true }
		})
	}

	onMount(() => {
		// Dev-only route. Bounce to home when the global mode gate is closed.
		enabled = isGlobalAiEnabled()
		if (!enabled) {
			goto('/')
			return
		}

		void loadDrafts()
		// Drafts live in the DB; poll so the inspector reflects writes from this
		// and other tabs (same-tab saves emit no DOM event).
		const interval = window.setInterval(() => void loadDrafts(), 1000)
		return () => window.clearInterval(interval)
	})

	function draftKey(row: DraftRow): string {
		return `${row.typ}:${row.path}`
	}

	async function deleteDraft(row: DraftRow) {
		if (!$workspaceStore) return
		await removeDraft($workspaceStore, row)
		await loadDrafts()
	}

	async function clearAll() {
		const ws = $workspaceStore
		if (!ws) return
		await Promise.all(drafts.map((row) => removeDraft(ws, row)))
		await loadDrafts()
	}
</script>

{#if enabled}
	<div class="p-6 max-w-5xl mx-auto">
		<div class="flex items-center justify-between mb-6">
			<div>
				<h1 class="text-2xl font-semibold">Global drafts</h1>
				<p class="text-sm text-tertiary">
					Dev-only inspector for the current user's DB-backed drafts.
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
			<ul class="space-y-2">
				{#each drafts as draft (draftKey(draft))}
					<li class="border rounded p-3 flex items-center justify-between gap-2">
						<div class="font-mono text-sm min-w-0 truncate">
							<span class="font-semibold">{draft.typ}</span>
							<span class="text-tertiary">·</span>
							<span>{draft.path}</span>
							<span class="text-tertiary text-xs ml-2">{draft.saved_at}</span>
						</div>
						<Button
							variant="default"
							startIcon={{ icon: Trash2 }}
							iconOnly
							onclick={() => deleteDraft(draft)}
						/>
					</li>
				{/each}
			</ul>
		{/if}
	</div>
{/if}
