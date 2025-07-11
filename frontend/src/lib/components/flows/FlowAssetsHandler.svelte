<script lang="ts" module>
	export function initFlowGraphAssetsCtx(): FlowGraphAssetContext {
		let s = $state({
			val: {
				assetsMap: {},
				selectedAsset: undefined,
				dbManagerDrawer: undefined,
				s3FilePicker: undefined,
				resourceEditorDrawer: undefined,
				resourceMetadataCache: {}
			}
		})
		return s
	}
</script>

<script lang="ts">
	import { inferAssets } from '$lib/infer'
	import { assetEq } from '../assets/lib'
	import OnChange from '../common/OnChange.svelte'
	import { getAllModules } from './flowExplorer'
	import { getContext } from 'svelte'
	import type { FlowGraphAssetContext } from './types'
	import { AssetService, ResourceService, type AssetUsageKind, type FlowModule } from '$lib/gen'
	import { deepEqual } from 'fast-equals'
	import { workspaceStore } from '$lib/stores'
	import S3FilePicker from '../S3FilePicker.svelte'
	import DbManagerDrawer from '../DBManagerDrawer.svelte'
	import ResourceEditorDrawer from '../ResourceEditorDrawer.svelte'

	let {
		modules
	}: {
		modules: FlowModule[]
	} = $props()

	const flowGraphAssetsCtx = getContext<FlowGraphAssetContext>('FlowGraphAssetContext')

	const assetsMap = $derived(flowGraphAssetsCtx.val.assetsMap)

	// Fetch resource metadata for the ExploreAssetButton
	const resMetadataCache = $derived(flowGraphAssetsCtx.val.resourceMetadataCache)
	$effect(() => {
		for (const asset of Object.values(assetsMap ?? []).flatMap((x) => x)) {
			if (asset.kind !== 'resource' || asset.path in resMetadataCache) continue
			resMetadataCache[asset.path] = undefined // avoid fetching multiple times because of async
			ResourceService.getResource({ path: asset.path, workspace: $workspaceStore! }).then(
				(r) => (resMetadataCache[asset.path] = { resource_type: r.resource_type })
			)
		}
	})

	// Fetch transitive assets (path scripts and flows)
	$effect(() => {
		if (!$workspaceStore) return
		let usages: { path: string; kind: AssetUsageKind }[] = []
		let modIds: string[] = []
		for (const mod of getAllModules(modules)) {
			if (mod.id in assetsMap) continue
			assetsMap[mod.id] = [] // avoid fetching multiple times because of async
			if (mod.value.type === 'flow' || mod.value.type === 'script') {
				usages.push({ path: mod.value.path, kind: mod.value.type })
				modIds.push(mod.id)
			}
		}
		if (usages.length) {
			AssetService.listAssetsByUsage({
				workspace: $workspaceStore,
				requestBody: { usages }
			}).then((result) => {
				result.forEach((assets, idx) => {
					assetsMap[modIds[idx]] = assets
				})
			})
		}
	})

	// Prune assetsMap to only contain assets that are actually used
	$effect(() => {
		const allModules = new Set(getAllModules(modules).map((mod) => mod.id))
		for (const modId in assetsMap) {
			if (modId !== 'Input' && !allModules.has(modId)) delete assetsMap[modId]
		}
	})
</script>

{#each getAllModules(modules) as mod (mod.id)}
	{#if mod.value.type === 'rawscript'}
		{@const v = mod.value}
		<OnChange
			key={[v.content, v.asset_fallback_access_types]}
			runFirstEffect
			onChange={() =>
				inferAssets(v.language, v.content)
					.then((assets) => {
						for (const override of v.asset_fallback_access_types ?? []) {
							assets = assets.map((asset) => {
								if (assetEq(asset, override) && !asset.access_type)
									return { ...asset, access_type: override.access_type }
								return asset
							})
						}
						if (assetsMap && !deepEqual(assetsMap[mod.id], assets)) assetsMap[mod.id] = assets
					})
					.catch((e) => {})}
		/>
	{/if}
{/each}

<S3FilePicker bind:this={flowGraphAssetsCtx.val.s3FilePicker} readOnlyMode />
<DbManagerDrawer bind:this={flowGraphAssetsCtx.val.dbManagerDrawer} />
<ResourceEditorDrawer bind:this={flowGraphAssetsCtx.val.resourceEditorDrawer} />
