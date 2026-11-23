<script lang="ts">
	import { workspaceStore, userWorkspaces } from '$lib/stores'
	import { workspaceIsFork } from '$lib/utils/workspaceHierarchy'
	import { Button } from './common'
	import { Pencil } from 'lucide-svelte'
	import { useWorkspaceDrafts } from '$lib/workspaceDrafts.svelte'
	import PageHeaderContent from '$lib/components/PageHeaderContent.svelte'
	import { pageHeader, PHONE_BAR } from '$lib/components/pageHeaderRegistry.svelte'
	import { onDestroy } from 'svelte'
	import {
		homeActionsGathered,
		homeCountLabels,
		homeHeaderMenu
	} from './home/homeHeaderMenu.svelte'

	// Surfaces pending drafts (scripts/flows/apps) for the current workspace and
	// links to the compare page in draft mode. Mutually exclusive with
	// ForkWorkspaceBanner: that one self-gates on `isFork`, this one on `!isFork`,
	// so a fork workspace never shows both. A fork counts its drafts in its changes button.
	let isFork = $derived(workspaceIsFork($workspaceStore, $userWorkspaces))

	// Count comes from the shared Workspace Drafts resource (count ≡ the draft
	// list; refreshes itself on deploy/discard). Pass undefined in a fork or with
	// no workspace so it doesn't fetch and the banner stays hidden.
	const drafts = useWorkspaceDrafts(() => (!isFork ? ($workspaceStore ?? undefined) : undefined))
	const draftCount = $derived(drafts.count)

	const draftCompareHref = $derived(
		$workspaceStore
			? '/forks/compare?workspace_id=' + encodeURIComponent($workspaceStore) + '&mode=draft'
			: undefined
	)
	const phone = $derived(pageHeader.barWidth > 0 && pageHeader.barWidth < PHONE_BAR)

	const menuId = homeHeaderMenu.register(-100, () =>
		!isFork && draftCount > 0 && homeActionsGathered()
			? [
					{
						displayName: `Review & deploy ${draftCount} draft${draftCount !== 1 ? 's' : ''}`,
						href: draftCompareHref
					}
				]
			: []
	)
	onDestroy(() => homeHeaderMenu.release(menuId))
</script>

{#if !isFork && draftCount > 0 && !homeActionsGathered()}
	<!-- In the band rather than a banner over the page: it is a standing fact about the workspace,
	     not an interruption, and one button says it and acts on it. -->
	<PageHeaderContent actions={draftsAction} actionsOrder={-100} />
{/if}

{#snippet draftsAction()}
	<!-- The count is the button, shaped like a fork's changes button: a count beside a button that
	     goes to the same place reads as two ways in. Fades in: the count lands after the bar it
	     joins has already painted. -->
	{@const label = `${draftCount} draft${draftCount !== 1 ? 's' : ''} not deployed yet: review and deploy`}
	<!-- Named by aria-label too: compact, the button's own text is a bare number. -->
	<Button
		btnClasses="wm-page-in tabular-nums"
		variant="default"
		unifiedSize="sm"
		href={draftCompareHref}
		title={label}
		aria-label={label}
	>
		<span class="flex items-center gap-1">
			<Pencil class="w-3 h-3 text-accent" />
			{draftCount}{phone || !homeCountLabels() ? '' : ` draft${draftCount !== 1 ? 's' : ''}`}
		</span>
	</Button>
{/snippet}
