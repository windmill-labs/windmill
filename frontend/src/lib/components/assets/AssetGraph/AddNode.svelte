<script lang="ts">
	import AddDataSourceMenu from './AddDataSourceMenu.svelte'
	import { Plus } from 'lucide-svelte'
	import { NODE } from '$lib/components/graph/util'

	interface Props {
		data: Pick<
			import('svelte').ComponentProps<typeof AddDataSourceMenu>,
			'onAddPipelineScript' | 'pathPrefix' | 'defaultPathSuffix'
		>
	}
	let { data }: Props = $props()
</script>

<!-- The layout gives this node a full node-width slot; centering the pill in
     it lines it up with the nodes below whatever its label's width. -->
<div class="flex justify-center" style="width: {NODE.width}px;">
	<AddDataSourceMenu
		onAddPipelineScript={data.onAddPipelineScript}
		pathPrefix={data.pathPrefix}
		defaultPathSuffix={data.defaultPathSuffix}
	>
		{#snippet trigger()}
			<!-- Quiet insert affordance, mirroring the flow editor's inline +
			     buttons (bg-surface + gray border + secondary text) — a filled
			     accent pill would outweigh every real node on the canvas. -->
			<button
				type="button"
				class="h-8 px-3 rounded-full flex items-center gap-1.5 whitespace-nowrap bg-surface border border-gray-400 dark:border-gray-600 text-xs font-normal text-secondary shadow-sm hover:bg-surface-hover transition-colors cursor-pointer"
			>
				<Plus size={16} class="shrink-0" />
				Add data source
			</button>
		{/snippet}
	</AddDataSourceMenu>
</div>
