<script lang="ts">
	import type { Snippet } from 'svelte'
	import ContextMenu, {
		type ContextMenuItem
	} from '$lib/components/common/contextmenu/ContextMenu.svelte'

	let { items, children }: { items: ContextMenuItem[]; children: Snippet } = $props()
</script>

<!-- Without items (outside the editor) the browser's own menu stays, rather than an empty one -->
{#if items.length}
	<ContextMenu {items}>{@render children()}</ContextMenu>
{:else}
	{@render children()}
{/if}
