<script lang="ts">
	import { ExternalLink } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import { roleCapabilities } from '$lib/stores'
	import { useOperatingWorkspaceHref } from './operatingWorkspace.svelte'

	interface Props {
		label?: string
		/** Called when the user comes back to this tab after following the link, so the
		 *  caller can reload the groups it lists. */
		onReturn?: () => void
	}

	let { label = 'Manage groups', onReturn }: Props = $props()

	// Inside a fork's editor the pickers list the fork's groups, not the navigated workspace's.
	const operatingHref = useOperatingWorkspaceHref()
	let followed = false
</script>

<svelte:window
	onfocus={() => {
		if (!followed) return
		followed = false
		onReturn?.()
	}}
/>

<!-- Groups have no entry in the main menu, so every place that picks one links to their page.
     It opens in a new tab: the callers are forms whose unsaved state a navigation would drop. -->
{#if $roleCapabilities.visibility.groups}
	<Button
		variant="subtle"
		unifiedSize="xs"
		href={operatingHref('/groups')}
		target="_blank"
		endIcon={{ icon: ExternalLink }}
		onClick={() => (followed = true)}
	>
		{label}
	</Button>
{/if}
