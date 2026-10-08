<!--
@component
The inside of one breadcrumb part: an optional leading icon and the label.

Every part of the header's breadcrumb renders this, whether its dropdown is a melt menu or a
popover, so they share one look. Separators are the host's job — a part must not contain the
slash before it, or hovering the part would highlight the separator too.
-->
<script lang="ts">
	import type { Snippet } from 'svelte'

	let { label, icon }: { label: string; icon?: Snippet } = $props()
</script>

{@render icon?.()}
<span class="truncate">{label}</span>
