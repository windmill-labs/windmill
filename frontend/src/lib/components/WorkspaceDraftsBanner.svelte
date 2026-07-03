<script lang="ts">
	import { workspaceStore, userWorkspaces } from '$lib/stores'
	import { workspaceIsFork } from '$lib/utils/workspaceHierarchy'
	import { Button } from './common'
	import { Pencil } from 'lucide-svelte'
	import { goto } from '$app/navigation'
	import { useWorkspaceDrafts } from '$lib/workspaceDrafts.svelte'

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
	<div class="w-full bg-blue-50 dark:bg-blue-900 text-xs rounded-b-md max-w-7xl mx-auto">
		<div class="px-4 py-2">
			<div class="flex items-center justify-between">
				<div class="flex items-center gap-3">
					<Pencil class="w-4 h-4 text-accent" />
					<span class="text-xs font-medium text-blue-900 dark:text-blue-100">
						This workspace has {draftCount} draft{draftCount !== 1 ? 's' : ''}
					</span>
				</div>
				<!-- Same button as the sibling ForkWorkspaceBanner CTA (they sit on the
				     same home page), kept visually identical on purpose. -->
				<Button variant="default" unifiedSize="sm" onclick={openDraftCompare}>
					Review & deploy drafts
				</Button>
			</div>
		</div>
	</div>
{/if}
