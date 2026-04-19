<script lang="ts">
	import { workspaceStore, userStore } from '$lib/stores'
	import { base } from '$lib/base'
	import Button from '$lib/components/common/button/Button.svelte'
	import AssetGraphCanvas from '$lib/components/assets/AssetGraph/AssetGraphCanvas.svelte'
	import type { AssetGraphResponse } from '$lib/components/assets/AssetGraph/types'
	import { ArrowLeft, Loader2, NetworkIcon, RefreshCw } from 'lucide-svelte'
	import { OpenAPI } from '$lib/gen'

	let loading = $state(true)
	let error = $state<string | null>(null)
	let graph = $state<AssetGraphResponse | null>(null)

	async function load() {
		if (!$workspaceStore) return
		loading = true
		error = null
		try {
			const base_url = OpenAPI.BASE ?? ''
			const res = await fetch(`${base_url}/api/w/${$workspaceStore}/assets/graph`, {
				credentials: 'include'
			})
			if (!res.ok) {
				throw new Error(`GET /assets/graph → ${res.status}`)
			}
			graph = (await res.json()) as AssetGraphResponse
		} catch (e) {
			error = e instanceof Error ? e.message : String(e)
		} finally {
			loading = false
		}
	}

	$effect(() => {
		if ($workspaceStore) load()
	})
</script>

<svelte:head>
	<title>Asset graph — Windmill</title>
</svelte:head>

{#if $userStore?.operator}
	<div class="p-8 text-tertiary">Page not available for operators.</div>
{:else}
	<div class="flex flex-col h-[calc(100vh-3rem)]">
		<div
			class="flex items-center justify-between gap-4 px-4 py-2 border-b border-gray-200 dark:border-gray-800"
		>
			<div class="flex items-center gap-3">
				<Button
					variant="subtle"
					unifiedSize="sm"
					href="{base}/assets"
					startIcon={{ icon: ArrowLeft }}
				>
					Assets
				</Button>
				<div class="flex items-center gap-2">
					<NetworkIcon size={18} class="text-tertiary" />
					<h1 class="text-lg font-semibold">Asset graph</h1>
				</div>
				<span class="text-xs text-tertiary">
					Workspace-wide view of assets and their producers/consumers.
				</span>
			</div>
			<div class="flex items-center gap-2">
				{#if graph}
					<span class="text-xs text-tertiary">
						{graph.assets.length} assets · {graph.runnables.length} runnables · {graph.edges.length}
						edges
					</span>
				{/if}
				<Button
					variant="subtle"
					unifiedSize="sm"
					startIcon={{ icon: RefreshCw }}
					onclick={load}
					disabled={loading}
				>
					Refresh
				</Button>
			</div>
		</div>

		<div class="flex-1 relative">
			{#if loading}
				<div class="absolute inset-0 flex items-center justify-center gap-2 text-tertiary">
					<Loader2 size={18} class="animate-spin" />
					<span>Loading graph…</span>
				</div>
			{:else if error}
				<div class="absolute inset-0 flex items-center justify-center text-red-500 text-sm">
					Failed to load graph: {error}
				</div>
			{:else if graph && graph.assets.length === 0 && graph.runnables.length === 0}
				<div class="absolute inset-0 flex items-center justify-center text-tertiary text-sm">
					No assets are referenced by scripts or flows in this workspace yet.
				</div>
			{:else if graph}
				<AssetGraphCanvas {graph} />
			{/if}
		</div>
	</div>
{/if}
