<script lang="ts">
	import { Handle, Position } from '@xyflow/svelte'
	import { twMerge } from 'tailwind-merge'
	import AssetGenericIcon from '$lib/components/icons/AssetGenericIcon.svelte'
	import { formatShortAssetPath, type AssetKind } from '$lib/components/assets/lib'
	import { NODE } from '$lib/components/graph/util'
	import PipelineInsertMenu, { type PipelineInsertPick } from './PipelineInsertMenu.svelte'
	import { Code2, Plus } from 'lucide-svelte'
	import type { ScriptLang } from '$lib/gen'

	interface Props {
		data: {
			asset_kind: AssetKind
			path: string
			onAddScript?: (
				asset: { kind: AssetKind; path: string },
				language: ScriptLang,
				scriptPath: string
			) => void
			pathPrefix?: string
			defaultPathSuffix?: string
		}
	}
	let { data }: Props = $props()

	let asset = $derived({ kind: data.asset_kind, path: data.path })

	const LANGUAGES: Array<{ label: string; lang: ScriptLang }> = [
		{ label: 'TypeScript (Bun)', lang: 'bun' },
		{ label: 'TypeScript (Deno)', lang: 'deno' },
		{ label: 'Python', lang: 'python3' },
		{ label: 'PostgreSQL', lang: 'postgresql' },
		{ label: 'DuckDB', lang: 'duckdb' },
		{ label: 'BigQuery', lang: 'bigquery' },
		{ label: 'Snowflake', lang: 'snowflake' },
		{ label: 'MySQL', lang: 'mysql' },
		{ label: 'MS SQL', lang: 'mssql' },
		{ label: 'Bash', lang: 'bash' },
		{ label: 'Go', lang: 'go' }
	]

	function handlePick(pick: PipelineInsertPick) {
		if (pick.kindId === 'materializer' && pick.language && pick.path) {
			data.onAddScript?.(
				{ kind: data.asset_kind, path: data.path },
				pick.language as ScriptLang,
				pick.path
			)
		}
	}

	let showAdd = $derived(data.onAddScript != undefined)
</script>

<div class="relative">
	<div
		class={twMerge(
			'flex items-center rounded-md drop-shadow-sm overflow-hidden border',
			'bg-surface-secondary border-gray-300 dark:border-gray-600 hover:border-blue-400 transition-colors'
		)}
		style="width: {NODE.width}px; min-height: {NODE.height}px;"
		title={data.path}
	>
		<AssetGenericIcon
			assetKind={data.asset_kind}
			class="shrink-0 ml-2 mr-2 text-blue-600 dark:text-blue-400"
			size="14px"
		/>
		<div class="flex flex-col min-w-0 flex-1 pr-2 py-0.5 leading-tight">
			<span class="text-3xs uppercase tracking-wide text-tertiary truncate">{data.asset_kind}</span>
			<span class="text-2xs font-mono text-emphasis truncate">{formatShortAssetPath(asset)}</span>
		</div>
	</div>
	{#if showAdd}
		<!-- Always-visible + below the asset for downstream materializer
		     creation. Half-overlapping the bottom edge so it visually attaches
		     to the node like the flow editor's between-step inserter. -->
		<div class="absolute left-1/2 -bottom-3 -translate-x-1/2 z-10">
			<PipelineInsertMenu
				kinds={[
					{
						id: 'materializer',
						label: 'Add downstream materializer',
						description: 'Triggered when this asset changes',
						icon: Code2,
						pickLanguage: true
					}
				]}
				languages={LANGUAGES as any}
				pathPrefix={data.pathPrefix ?? ''}
				defaultPathSuffix={data.defaultPathSuffix ?? ''}
				onPick={handlePick}
			>
				{#snippet trigger()}
					<button
						type="button"
						onclick={(e) => e.stopPropagation()}
						class="bg-emerald-500 hover:bg-emerald-600 text-white rounded-full w-5 h-5 flex items-center justify-center shadow border-2 border-surface-secondary"
						title="Add downstream materializer"
					>
						<Plus size={12} />
					</button>
				{/snippet}
			</PipelineInsertMenu>
		</div>
	{/if}
</div>

<Handle type="target" position={Position.Top} isConnectable={false} />
<Handle type="source" position={Position.Bottom} isConnectable={false} />
