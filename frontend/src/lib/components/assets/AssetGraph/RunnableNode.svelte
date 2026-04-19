<script lang="ts">
	import { Handle, Position } from '@xyflow/svelte'
	import { Code2, GitBranch, ExternalLink } from 'lucide-svelte'
	import { base } from '$lib/base'
	import type { GraphUsageKind } from './types'

	interface Props {
		data: { runnable_kind: GraphUsageKind; path: string }
	}
	let { data }: Props = $props()

	let href = $derived(
		data.runnable_kind === 'flow'
			? `${base}/flows/edit/${data.path}`
			: `${base}/scripts/edit/${data.path}`
	)
	let Icon = $derived(data.runnable_kind === 'flow' ? GitBranch : Code2)
	let label = $derived(data.runnable_kind === 'flow' ? 'FLOW' : 'SCRIPT')
</script>

<div
	class="bg-surface-secondary border border-gray-300 dark:border-gray-700 rounded-md shadow-sm px-3 py-2 w-[260px] hover:border-emerald-500 transition-colors"
>
	<Handle type="target" position={Position.Left} class="!bg-emerald-500" />
	<div class="flex items-center gap-2">
		<Icon size={16} class="text-emerald-700 dark:text-emerald-400 shrink-0" />
		<div class="flex flex-col min-w-0 flex-1">
			<span class="text-[10px] uppercase tracking-wide text-tertiary">{label}</span>
			<span class="text-xs font-mono truncate" title={data.path}>{data.path}</span>
		</div>
		<a
			{href}
			target="_blank"
			class="text-tertiary hover:text-primary"
			onclick={(e) => e.stopPropagation()}
			aria-label="Open {label.toLowerCase()} editor"
		>
			<ExternalLink size={14} />
		</a>
	</div>
	<Handle type="source" position={Position.Right} class="!bg-emerald-500" />
</div>
