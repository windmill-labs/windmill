<script lang="ts">
	import { Handle, Position } from '@xyflow/svelte'
	import { Code2, GitBranch, Sparkles } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import type { GraphUsageKind } from './types'
	import { NODE } from '$lib/components/graph/util'

	interface Props {
		data: { runnable_kind: GraphUsageKind; path: string; is_materializer?: boolean }
	}
	let { data }: Props = $props()

	let Icon = $derived(data.runnable_kind === 'flow' ? GitBranch : Code2)
	let label = $derived(
		data.is_materializer ? 'materializer' : data.runnable_kind === 'flow' ? 'flow' : 'script'
	)
</script>

<div class="relative">
	<div
		class={twMerge(
			'flex items-center rounded-md drop-shadow-sm overflow-hidden border',
			'bg-surface-tertiary border-gray-300 dark:border-gray-600 hover:border-emerald-500 transition-colors',
			data.is_materializer && 'border-emerald-400/60'
		)}
		style="width: {NODE.width}px; min-height: {NODE.height}px;"
		title={data.path}
	>
		<Icon size={14} class="shrink-0 ml-2 mr-2 text-emerald-700 dark:text-emerald-400" />
		<div class="flex flex-col min-w-0 flex-1 pr-2 py-0.5 leading-tight">
			<span class="text-3xs uppercase tracking-wide text-tertiary truncate">{label}</span>
			<span class="text-2xs font-mono text-emphasis truncate">{data.path}</span>
		</div>
		{#if data.is_materializer}
			<div
				class="shrink-0 flex items-center gap-0.5 px-1.5 py-0.5 mr-1.5 rounded-sm bg-emerald-100 dark:bg-emerald-900/40 text-emerald-700 dark:text-emerald-300"
				title="// materialize — pipeline member"
			>
				<Sparkles size={10} />
			</div>
		{/if}
	</div>
</div>

<Handle type="target" position={Position.Top} isConnectable={false} />
<Handle type="source" position={Position.Bottom} isConnectable={false} />
