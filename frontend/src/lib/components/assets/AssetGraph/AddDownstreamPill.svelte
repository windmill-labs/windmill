<!--
@component
The "+" on an asset's bottom edge: a circle at rest, widened into a labelled pill
while its menu is open or while it is being dragged. The label animates its
max-width (width can't transition to auto) and never wraps.
-->
<script lang="ts">
	import { Plus } from 'lucide-svelte'

	let { expanded }: { expanded: boolean } = $props()
</script>

<span
	class="group/add h-6 min-w-6 rounded-full flex items-center justify-center bg-surface border border-gray-400 dark:border-gray-600 text-secondary hover:text-primary hover:border-gray-500 shadow-sm leading-none overflow-hidden transition-[padding] duration-200 ease-out data-[open=true]:pl-1"
	data-open={expanded}
>
	<Plus size={16} strokeWidth={2.5} class="shrink-0" />
	<span
		class="max-w-0 opacity-0 whitespace-nowrap text-3xs font-normal transition-[max-width,opacity,margin] duration-200 ease-out group-data-[open=true]/add:max-w-32 group-data-[open=true]/add:opacity-100 group-data-[open=true]/add:ml-1 group-data-[open=true]/add:mr-2"
	>
		Add downstream step
	</span>
</span>
