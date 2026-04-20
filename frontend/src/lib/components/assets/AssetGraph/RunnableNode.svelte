<script lang="ts">
	import { Handle, Position } from '@xyflow/svelte'
	import { Code2, GitBranch } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import type { GraphUsageKind } from './types'
	import { NODE } from '$lib/components/graph/util'

	interface Props {
		data: { runnable_kind: GraphUsageKind; path: string }
	}
	let { data }: Props = $props()

	let Icon = $derived(data.runnable_kind === 'flow' ? GitBranch : Code2)
	let label = $derived(data.runnable_kind === 'flow' ? 'flow' : 'script')
</script>

<div class="relative">
	<div
		class={twMerge(
			'flex items-center rounded-md drop-shadow-sm overflow-hidden',
			'bg-surface-tertiary outline outline-1 outline-transparent hover:outline-emerald-500 transition-colors'
		)}
		style="width: {NODE.width}px; min-height: {NODE.height + 30}px;"
		title={data.path}
	>
		<Icon size={16} class="shrink-0 ml-2 mr-2 text-emerald-700 dark:text-emerald-400" />
		<div class="flex flex-col min-w-0 flex-1 pr-2 py-1.5">
			<span class="text-3xs uppercase tracking-wide text-tertiary truncate">{label}</span>
			<span class="text-2xs font-mono text-emphasis truncate">{data.path}</span>
		</div>
	</div>
</div>

<Handle type="target" position={Position.Top} isConnectable={false} />
<Handle type="source" position={Position.Bottom} isConnectable={false} />
