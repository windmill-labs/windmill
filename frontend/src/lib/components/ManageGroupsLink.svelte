<script lang="ts">
	import type { Snippet } from 'svelte'
	import { fromStore } from 'svelte/store'
	import { ExternalLink } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import { capabilitiesInWorkspace, useRoleCapabilities } from '$lib/operatorWriteRights'
	import { useOperatingWorkspace } from './operatingWorkspace.svelte'
	import { withWorkspaceParam } from './sessions/sessionMode.svelte'

	interface Props {
		label?: string
		/** The workspace whose groups the caller lists, when it is not the operating one. */
		workspace?: string
		/** Called when the user comes back to this tab after following the link, so the
		 *  caller can reload the groups it lists. */
		onReturn?: () => void
		/** Text shown before the link, hidden with it. */
		children?: Snippet
	}

	let { label = 'Manage groups', workspace, onReturn, children }: Props = $props()

	// Inside a fork's editor the pickers list the fork's groups, not the navigated workspace's.
	const operating = fromStore(useOperatingWorkspace())
	const operatingCaps = fromStore(useRoleCapabilities())
	const target = $derived(workspace ?? operating.current)
	const canSeeGroups = $derived(
		(workspace ? capabilitiesInWorkspace(workspace) : operatingCaps.current).visibility.groups
	)
	let followed = false
</script>

<svelte:window
	onfocus={() => {
		if (!followed) return
		followed = false
		onReturn?.()
	}}
/>

{#snippet link()}
	<Button
		variant="subtle"
		unifiedSize="xs"
		href={withWorkspaceParam('/groups', target)}
		target="_blank"
		endIcon={{ icon: ExternalLink }}
		onClick={() => (followed = true)}
	>
		{label}
	</Button>
{/snippet}

<!-- Groups have no entry in the main menu, so every place that picks one links to their page.
     It opens in a new tab: the callers are forms whose unsaved state a navigation would drop. -->
{#if canSeeGroups}
	{#if children}
		<div class="flex items-center gap-2 text-xs text-secondary">
			{@render children()}
			{@render link()}
		</div>
	{:else}
		{@render link()}
	{/if}
{/if}
