<script lang="ts">
	import { Tab, Tabs } from '$lib/components/common'
	import { goto } from '$lib/navigation'
	import { roleCapabilities } from '$lib/stores'

	// Folders and Groups read as one Sharing page: the sidebar links to it once and these tabs
	// move between the two routes, which stay separate so links to either keep working.
	let { selected }: { selected: 'folders' | 'groups' } = $props()

	const bothShown = $derived(
		$roleCapabilities.visibility.folders && $roleCapabilities.visibility.groups
	)
</script>

{#if bothShown}
	<Tabs {selected} on:selected={(e) => goto(`/${e.detail}`)} wrapperClass="pt-4">
		<Tab value="folders" label="Folders" />
		<Tab value="groups" label="Groups" />
	</Tabs>
{/if}
