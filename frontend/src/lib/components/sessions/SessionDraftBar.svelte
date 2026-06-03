<script lang="ts">
	import { onMount } from 'svelte'
	import { Pencil } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import { userWorkspaces } from '$lib/stores'
	import { goto } from '$lib/navigation'
	import { getRuntime } from './sessionRuntime.svelte'
	import type { Session } from './sessionState.svelte'
	import DraftDiffDrawer from './DraftDiffDrawer.svelte'
	import SessionDiffButton from './SessionDiffButton.svelte'

	let { session }: { session: Session } = $props()

	// Only meaningful once the session committed to a workspace (post first
	// send). Drafts are read from that committed workspace's `draft` table — the
	// same source as the compare page this bar links to. Independent of the fork
	// bar: a fork session with drafts shows both.
	const committedId = $derived(session.workspace_id)
	const sessionWorkspace = $derived(
		committedId ? $userWorkspaces.find((w) => w.id === committedId) : undefined
	)
	const available = $derived(!!committedId && !!sessionWorkspace)

	const runtime = $derived(getRuntime(session.id))
	const count = $derived(runtime?.draftCount.val ?? 0)

	// Initial fetch (deduped) once committedId/workspace are known. Must NOT call
	// refreshDraftCount here: it reads loadingDraftCount ($state), which this
	// reactive effect would then track and refreshDraftCount mutates → infinite
	// loop. ensureDraftCount's early-return reads only the plain (non-reactive)
	// key, so it's safe in an effect.
	$effect(() => {
		if (!runtime || !committedId || !available) return
		void runtime.ensureDraftCount(committedId)
	})

	// The runtime persists across client-side navigation, so ensureDraftCount()
	// dedupes and would keep a stale count (e.g. a 0 cached before a draft was
	// created) when the session is re-opened. Force one fresh fetch on each mount
	// — onMount is non-reactive, so it can't loop the way the effect above would.
	onMount(() => {
		if (runtime && committedId) void runtime.refreshDraftCount(committedId)
	})

	// Refresh when the AI finishes a turn — a deploy promotes a draft (the
	// server draft is deleted), so the count must reflect it immediately.
	let wasLoading = $state(false)
	$effect(() => {
		const isLoading = runtime?.manager.loading ?? false
		if (wasLoading && !isLoading) void runtime?.refreshDraftCount()
		wasLoading = isLoading
	})

	// Refresh when the tab regains visibility — covers drafts saved in another
	// tab or by another user while we were away.
	$effect(() => {
		if (!runtime || !committedId || !available) return
		function onVisibilityChange() {
			if (document.visibilityState !== 'visible') return
			void runtime?.refreshDraftCount()
		}
		document.addEventListener('visibilitychange', onVisibilityChange)
		return () => document.removeEventListener('visibilitychange', onVisibilityChange)
	})

	let drawer: DraftDiffDrawer | undefined = $state(undefined)

	function openReview() {
		if (!committedId) return
		goto(`/forks/compare?workspace_id=${encodeURIComponent(committedId)}&mode=draft`)
	}
</script>

{#if available && count > 0 && committedId}
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
