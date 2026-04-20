<script lang="ts">
	import { workspaceStore, userStore } from '$lib/stores'
	import { base } from '$lib/base'
	import Button from '$lib/components/common/button/Button.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import AssetGraphCanvas from '$lib/components/assets/AssetGraph/AssetGraphCanvas.svelte'
	import AssetGraphDetailsPane from '$lib/components/assets/AssetGraph/AssetGraphDetailsPane.svelte'
	import type {
		AssetGraphResponse,
		AssetGraphSelection
	} from '$lib/components/assets/AssetGraph/types'
	import { ArrowLeft, Loader2, NetworkIcon, RefreshCw } from 'lucide-svelte'
	import { OpenAPI } from '$lib/gen'
	import { resource } from 'runed'
	import { Pane, Splitpanes } from 'svelte-splitpanes'

	// Variables and resources tend to be hubs (DB creds, API keys) used by
	// most runnables, so they swamp the layout. Hidden by default; the
	// toggle in the header opts back in.
	const DATA_KINDS = ['s3object', 'ducklake', 'datatable', 'volume']

	let includeConfigKinds = $state(false)
	let selection = $state<AssetGraphSelection | undefined>(undefined)

	let graphRes = resource(
		[() => $workspaceStore, () => includeConfigKinds],
		async ([ws, includeAll], _prev, { signal }) => {
			if (!ws) return undefined
			const base_url = OpenAPI.BASE ?? ''
			const qs = includeAll ? '' : `?asset_kinds=${DATA_KINDS.join(',')}`
			const res = await fetch(`${base_url}/w/${ws}/assets/graph${qs}`, {
				credentials: 'include',
				signal
			})
			if (!res.ok) throw new Error(`GET /assets/graph → ${res.status}`)
			return (await res.json()) as AssetGraphResponse
		}
	)

	// Pluralize the kind label: 1 script, 2 scripts, 1 s3object, 2 s3objects.
	function pluralize(n: number, singular: string): string {
		return `${n} ${singular}${n === 1 ? '' : 's'}`
	}

	let summary = $derived.by<string[]>(() => {
		const g = graphRes.current
		if (!g) return []
		const parts: string[] = []
		const scripts = g.runnables.filter((r) => r.usage_kind === 'script').length
		const flows = g.runnables.filter((r) => r.usage_kind === 'flow').length
		if (scripts) parts.push(pluralize(scripts, 'script'))
		if (flows) parts.push(pluralize(flows, 'flow'))
		const byKind = new Map<string, number>()
		for (const a of g.assets) byKind.set(a.kind, (byKind.get(a.kind) ?? 0) + 1)
		for (const [kind, n] of byKind) parts.push(pluralize(n, kind))
		return parts
	})
</script>

<svelte:head>
	<title>Asset graph — Windmill</title>
</svelte:head>

{#if $userStore?.operator}
	<div class="p-8 text-tertiary">Page not available for operators.</div>
{:else}
	<div class="flex flex-col h-full">
		<div
			class="border-b flex flex-row justify-between gap-2 px-2 py-1 items-center overflow-y-visible overflow-x-auto min-h-10 shrink-0 whitespace-nowrap"
		>
			<div class="flex flex-row items-center gap-2">
				<Button
					variant="subtle"
					unifiedSize="sm"
					href="{base}/assets"
					startIcon={{ icon: ArrowLeft }}
					iconOnly
					title="Back to assets"
				/>
				<NetworkIcon size={16} class="text-tertiary shrink-0" />
				<h1 class="text-sm font-semibold">Asset graph</h1>
				{#if summary.length > 0}
					<span class="text-xs text-tertiary">· {summary.join(' · ')}</span>
				{/if}
			</div>
			<div class="flex flex-row items-center gap-2">
				<Toggle
					bind:checked={includeConfigKinds}
					size="xs"
					options={{ right: 'Vars & resources' }}
					disabled={graphRes.loading}
				/>
				<Button
					variant="subtle"
					unifiedSize="sm"
					startIcon={{ icon: RefreshCw }}
					onclick={() => graphRes.refetch()}
					disabled={graphRes.loading}
					iconOnly
					title="Refresh"
				/>
			</div>
		</div>

		<div class="flex-1 min-h-0">
			{#if graphRes.loading && !graphRes.current}
				<div class="h-full flex items-center justify-center gap-2 text-tertiary">
					<Loader2 size={18} class="animate-spin" />
					<span>Loading graph…</span>
				</div>
			{:else if graphRes.error}
				<div class="h-full flex items-center justify-center text-red-500 text-sm">
					Failed to load graph: {graphRes.error.message}
				</div>
			{:else if graphRes.current && graphRes.current.assets.length === 0 && graphRes.current.runnables.length === 0}
				<div class="h-full flex items-center justify-center text-tertiary text-sm">
					No assets are referenced by scripts or flows in this workspace yet.
				</div>
			{:else if graphRes.current}
				<Splitpanes class="!h-full">
					<Pane size={selection ? 60 : 100}>
						<AssetGraphCanvas
							graph={graphRes.current}
							{selection}
							onselect={(s) => (selection = s)}
						/>
					</Pane>
					{#if selection && $workspaceStore}
						<Pane size={40} minSize={25}>
							<AssetGraphDetailsPane
								{selection}
								workspace={$workspaceStore}
								onclose={() => (selection = undefined)}
							/>
						</Pane>
					{/if}
				</Splitpanes>
			{/if}
		</div>
	</div>
{/if}
