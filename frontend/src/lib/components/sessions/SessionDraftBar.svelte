<script lang="ts">
	import { Pencil } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import { goto } from '$lib/navigation'
	import type { Session } from './sessionState.svelte'
	import DraftDiffDrawer from './DraftDiffDrawer.svelte'
	import SessionDiffButton from './SessionDiffButton.svelte'
	import { useWorkspaceDrafts } from '$lib/workspaceDrafts.svelte'

	let { session }: { session: Session } = $props()

	// Only meaningful once the session committed to a workspace (post first send).
	// The Draft Count comes from the shared Workspace Drafts resource: it fetches
	// on mount and whenever a Server-Draft mutation invalidates the workspace, so
	// the count is fresh on every (re)open with no per-session caching.
	const committedId = $derived(session.workspace_id)
	const drafts = useWorkspaceDrafts(() => committedId)
	const count = $derived(drafts.count)

	let drawer: DraftDiffDrawer | undefined = $state(undefined)

	function openReview() {
		if (!committedId) return
		goto(`/forks/compare?workspace_id=${encodeURIComponent(committedId)}&mode=draft`)
	}
</script>

{#if committedId && count > 0}
	<div
		class="flex flex-row items-center justify-between gap-2 py-2 px-3 text-xs border rounded-md bg-surface-tertiary"
	>
		<div class="flex items-center gap-1.5 min-w-0">
			<span class="inline-flex shrink-0"><Pencil class="w-3.5 h-3.5 text-secondary" /></span>
			<span class="truncate text-secondary">{count} draft{count === 1 ? '' : 's'}</span>
		</div>
		<div class="flex items-center gap-1 shrink-0">
			<SessionDiffButton {count} onclick={() => drawer?.open()} />
			<Button variant="default" unifiedSize="xs" onclick={openReview}>Review</Button>
		</div>
	</div>

	<DraftDiffDrawer bind:this={drawer} workspaceId={committedId} />
{/if}
