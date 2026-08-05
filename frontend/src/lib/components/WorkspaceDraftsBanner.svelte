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
	<!-- Side padding mirrors the page content container below, so the banner
	     stays aligned with it instead of bleeding to the viewport edges. -->
	<div class="w-full text-xs max-w-7xl mx-auto px-4 sm:px-8 pt-2">
		<div class="bg-blue-50 dark:bg-blue-900 rounded-md px-4 py-2">
			<div class="flex flex-wrap items-center justify-between gap-x-3 gap-y-2">
				<div class="flex items-center gap-3 min-w-0">
					<Pencil class="w-4 h-4 text-accent shrink-0" />
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
