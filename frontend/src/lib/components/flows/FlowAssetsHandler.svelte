<script lang="ts" module>
	export function initFlowGraphAssetsCtx({
		getModules
	}: {
		getModules: () => FlowModule[]
	}): FlowGraphAssetContext {
		let s = $state({
			val: {
				selectedAsset: undefined,
				dbManagerDrawer: undefined,
				s3FilePicker: undefined,
				resourceEditorDrawer: undefined,
				resourceMetadataCache: {},
				additionalAssetsMap: {},
				computeAssetsCount: (asset) => {
					return getAllModules(getModules())
						.flatMap((m) => getFlowModuleAssets(m, s.val.additionalAssetsMap) ?? [])
						.filter((a) => assetEq(asset, a)).length
				}
			}
		} satisfies FlowGraphAssetContext)
		return s
	}
</script>

<script lang="ts">
	import { inferAssets } from '$lib/infer'
	import { assetEq, getFlowModuleAssets, type AssetWithAccessType } from '../assets/lib'
	import OnChange from '../common/OnChange.svelte'
	import { getAllModules } from './flowExplorer'
	import { getContext, untrack } from 'svelte'
	import type { FlowGraphAssetContext } from './types'
	import {
		AssetService,
		ResourceService,
		type AssetUsageKind,
		type FlowModule,
		type RawScript
	} from '$lib/gen'
	import { deepEqual } from 'fast-equals'
	import { workspaceStore } from '$lib/stores'
	import S3FilePicker from '../S3FilePicker.svelte'
	import DbManagerDrawer from '../DBManagerDrawer.svelte'
	import ResourceEditorDrawer from '../ResourceEditorDrawer.svelte'

	let {
		modules,
		enableParser = false,
		enableDbExplore = false,
		enablePathScriptAndFlowAssets = false
	}: {
		modules: FlowModule[]
		enableParser?: boolean
		enableDbExplore?: boolean
		enablePathScriptAndFlowAssets?: boolean
	} = $props()

	const flowGraphAssetsCtx = getContext<FlowGraphAssetContext | undefined>('FlowGraphAssetContext')

	let allModules = $derived(getAllModules(modules))

	// Fetch resource metadata for the ExploreAssetButton
	const resMetadataCache = $derived(flowGraphAssetsCtx?.val.resourceMetadataCache)
	$effect(() => {
		if (!resMetadataCache || !enableDbExplore) return
		const assets: AssetWithAccessType[] =
			allModules.flatMap(
				(m) => getFlowModuleAssets(m, flowGraphAssetsCtx?.val.additionalAssetsMap) ?? []
			) ?? []
		for (const asset of assets) {
			if (asset.kind !== 'resource' || asset.path in resMetadataCache) continue
			resMetadataCache[asset.path] = undefined // avoid fetching multiple times because of async
			ResourceService.getResource({ path: asset.path, workspace: $workspaceStore! })
				.then((r) => (resMetadataCache[asset.path] = { resource_type: r.resource_type }))
				.catch((err) => {
					console.error("Couldn't fetch resource", asset.path, err)
				})
		}
	})

	// Fetch transitive assets (path scripts and flows)
	$effect(() => {
		if (!$workspaceStore || !flowGraphAssetsCtx || !enablePathScriptAndFlowAssets) return
		let usages: { path: string; kind: AssetUsageKind }[] = []
		let modIds: string[] = []
		for (const mod of allModules) {
			if (mod.id in flowGraphAssetsCtx.val.additionalAssetsMap) continue
			flowGraphAssetsCtx.val.additionalAssetsMap[mod.id] = [] // avoid fetching multiple times because of async
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
					flowGraphAssetsCtx.val.additionalAssetsMap[modIds[idx]] = assets
				})
			})
		}
	})
	// Prune all additionalAssetsMap entries from deleted modules
	$effect(() => {
		if (!flowGraphAssetsCtx) return
		const modulesSet = new Set(allModules.map((m) => m.id))
		for (const key of Object.keys(flowGraphAssetsCtx.val.additionalAssetsMap)) {
			if (!modulesSet.has(key)) {
				delete flowGraphAssetsCtx.val.additionalAssetsMap[key]
			}
		}
	})

	async function parseAndUpdateRawScriptModule(v: RawScript) {
		try {
			let assets = await inferAssets(v.language, v.content)
			if (!deepEqual(v.assets, assets)) v.assets = assets
		} catch (e) {}
	}

	// Check for raw script modules whose assets were not parsed. Useful for flows created
	// before the assets feature was introduced.
	$effect(() => {
		if (!enableParser) return
		untrack(() => {
			setTimeout(() => {
				for (const mod of allModules) {
					if (mod.value.type === 'rawscript' && mod.value.assets === undefined) {
						console.log('RawScript module', mod.id, 'without assets field, parsing')
						parseAndUpdateRawScriptModule(mod.value)
					}
				}
			}, 500) // ensure modules are loaded
		})
	})
</script>

{#if enableParser}
	{#each allModules as mod (mod.id)}
		{#if mod.value.type === 'rawscript'}
			{@const v = mod.value}
			<OnChange key={v.content} onChange={() => parseAndUpdateRawScriptModule(v)} />
		{/if}
	{/each}
{/if}

{#if flowGraphAssetsCtx}
	<S3FilePicker bind:this={flowGraphAssetsCtx.val.s3FilePicker} readOnlyMode />
	<DbManagerDrawer bind:this={flowGraphAssetsCtx.val.dbManagerDrawer} />
	<ResourceEditorDrawer bind:this={flowGraphAssetsCtx.val.resourceEditorDrawer} />
{/if}
