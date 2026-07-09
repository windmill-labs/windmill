<script lang="ts">
	// A custom (`// data_test <script_path>`) test rendered as its own node,
	// hanging off the asset it validates. Clicking it opens the test script
	// (handled by AssetGraphCanvas.handleNodeClick, like any runnable). Styled
	// distinct from producer nodes — dashed, flask-marked — so it reads as a
	// check on the data, not a step that produces it.
	import { Handle, Position } from '@xyflow/svelte'
	import { NODE } from '$lib/components/graph/util'
	import { FlaskConical } from 'lucide-svelte'

	let { data, selected = false }: { data: { path: string }; selected?: boolean } = $props()
</script>

<Handle type="target" position={Position.Top} isConnectable={false} />
<div
	class="flex items-center rounded-md border border-dashed bg-surface-secondary transition-colors cursor-pointer
		{selected
		? 'border-border-selected'
		: 'border-gray-400 dark:border-gray-600 hover:border-gray-500'}"
	style="width: {NODE.width}px; min-height: 28px;"
	title={`data test: ${data.path}`}
>
	<FlaskConical
		size={12}
		class={`shrink-0 ml-2 mr-1.5 ${selected ? 'text-accent' : 'text-secondary'}`}
	/>
	<span class="text-3xs uppercase tracking-wide text-tertiary mr-1.5">test</span>
	<span class="flex-1 min-w-0 pr-2 py-0.5 text-2xs font-mono text-secondary truncate">
		{data.path}
	</span>
</div>
