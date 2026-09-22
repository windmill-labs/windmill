<!--
@component
Fills the layout's page header from the route that mounts it: the breadcrumb (an item's path, or a
section name for a list) and the page's own buttons. Renders nothing itself.

Only a route page mounts this. An editor nested in a session pane or a drawer must not take over
the header, so nesting is decided by the caller rather than guessed from context.
-->
<script lang="ts">
	import { onMount } from 'svelte'
	import { pageHeader, type PageHeaderContent } from './pageHeaderRegistry.svelte'

	let { item, section, actions }: PageHeaderContent = $props()

	onMount(() => {
		const id = pageHeader.register(() => ({ item, section, actions }))
		return () => pageHeader.release(id)
	})
</script>
