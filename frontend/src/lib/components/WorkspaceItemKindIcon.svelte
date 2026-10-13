<!--
@component
The glyph the rest of the app gives a workspace item's kind (see RowIcon), at the caller's size.
-->
<script lang="ts">
	import { Code2, LayoutDashboard } from 'lucide-svelte'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import type { WorkspaceItemKind } from '$lib/components/workspacePicker'

	let { kind, size = 14 }: { kind: WorkspaceItemKind; size?: number } = $props()
</script>

{#if kind === 'flow'}
	<BarsStaggered {size} class="flex-shrink-0 text-teal-500" />
{:else if kind === 'app'}
	<LayoutDashboard {size} class="flex-shrink-0 text-orange-500" />
{:else}
	<Code2 {size} class="flex-shrink-0 text-blue-500" />
{/if}
