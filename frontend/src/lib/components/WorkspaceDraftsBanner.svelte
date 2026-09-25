<script lang="ts">
	import { workspaceStore, userWorkspaces } from '$lib/stores'
	import { workspaceIsFork } from '$lib/utils/workspaceHierarchy'
	import { Badge, Button } from './common'
	import { goto } from '$app/navigation'
	import { useWorkspaceDrafts } from '$lib/workspaceDrafts.svelte'
	import PageHeaderContent from '$lib/components/PageHeaderContent.svelte'

	// Surfaces pending drafts (scripts/flows/apps) for the current workspace and
	// links to the compare page in draft mode. Mutually exclusive with
	// ForkWorkspaceBanner: that one self-gates on `isFork`, this one on `!isFork`,
	// so a fork workspace never shows both. In a fork, drafts are discovered via
	// the on-page "Deployed ↔ draft (N)" toggle badge instead.
	let isFork = $derived(workspaceIsFork($workspaceStore, $userWorkspaces))

	// Count comes from the shared Workspace Drafts resource (count ≡ the draft
	// list; refreshes itself on deploy/discard). Pass undefined in a fork or with
	// no workspace so it doesn't fetch and the banner stays hidden.
	const drafts = useWorkspaceDrafts(() => (!isFork ? ($workspaceStore ?? undefined) : undefined))
	const draftCount = $derived(drafts.count)

	function openDraftCompare() {
		if ($workspaceStore) {
			goto('/forks/compare?workspace_id=' + encodeURIComponent($workspaceStore) + '&mode=draft', {
				replaceState: true
			})
		}
	}
</script>

{#if !isFork && draftCount > 0}
	<!-- In the band rather than a banner over the page: it is a standing fact about the workspace,
	     not an interruption, and one button says it and acts on it. -->
	<PageHeaderContent actions={draftsAction} actionsOrder={-100} />
{/if}

{#snippet draftsAction()}
	<!-- A count on its own leaves the reader to work out what it counts; the sentence says it. -->
	<span class="shrink-0 text-2xs text-tertiary">This workspace has</span>
	<Badge color="blue" small>
		{draftCount} draft{draftCount !== 1 ? 's' : ''}
	</Badge>
	<Button variant="subtle" unifiedSize="sm" onclick={openDraftCompare}>
		Review & deploy drafts
	</Button>
{/snippet}
