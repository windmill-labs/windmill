<script lang="ts">
	import { Handle, Position } from '@xyflow/svelte'
	import { twMerge } from 'tailwind-merge'
	import AssetGenericIcon from '$lib/components/icons/AssetGenericIcon.svelte'
	import { formatShortAssetPath, type AssetKind } from '$lib/components/assets/lib'
	import { NODE } from '$lib/components/graph/util'

	interface Props {
		data: { asset_kind: AssetKind; path: string }
	}
	let { data }: Props = $props()

	let asset = $derived({ kind: data.asset_kind, path: data.path })
</script>

<div class="relative">
	<div
		class={twMerge(
			'flex items-center rounded-md drop-shadow-sm overflow-hidden',
			'bg-surface-secondary outline outline-1 outline-transparent hover:outline-blue-400 transition-colors'
		)}
		style="width: {NODE.width}px; min-height: {NODE.height + 30}px;"
		title={data.path}
	>
		<AssetGenericIcon
			assetKind={data.asset_kind}
			class="shrink-0 ml-2 mr-2 text-blue-600 dark:text-blue-400"
			size="16px"
		/>
		<div class="flex flex-col min-w-0 flex-1 pr-2 py-1.5">
			<span class="text-3xs uppercase tracking-wide text-tertiary truncate">{data.asset_kind}</span>
			<span class="text-2xs font-mono text-emphasis truncate">{formatShortAssetPath(asset)}</span>
		</div>
	</div>
</div>

<Handle type="target" position={Position.Top} isConnectable={false} />
<Handle type="source" position={Position.Bottom} isConnectable={false} />
