<script lang="ts">
	import { Handle, Position } from '@xyflow/svelte'
	import { Database, FileBox, Layers, HardDrive, KeyRound } from 'lucide-svelte'
	import type { AssetKind } from '$lib/gen'

	interface Props {
		data: { asset_kind: AssetKind; path: string }
	}
	let { data }: Props = $props()

	function iconFor(kind: AssetKind) {
		switch (kind) {
			case 's3object':
				return FileBox
			case 'resource':
				return KeyRound
			case 'ducklake':
				return Database
			case 'datatable':
				return Layers
			case 'volume':
				return HardDrive
			default:
				return FileBox
		}
	}
	let Icon = $derived(iconFor(data.asset_kind))
</script>

<div
	class="bg-surface border border-gray-300 dark:border-gray-700 rounded-md shadow-sm px-3 py-2 w-[260px] hover:border-blue-400 transition-colors"
>
	<Handle type="target" position={Position.Left} class="!bg-blue-500" />
	<div class="flex items-center gap-2">
		<Icon size={16} class="text-blue-600 dark:text-blue-400 shrink-0" />
		<div class="flex flex-col min-w-0 flex-1">
			<span class="text-[10px] uppercase tracking-wide text-tertiary">{data.asset_kind}</span>
			<span class="text-xs font-mono truncate" title={data.path}>{data.path}</span>
		</div>
	</div>
	<Handle type="source" position={Position.Right} class="!bg-blue-500" />
</div>
