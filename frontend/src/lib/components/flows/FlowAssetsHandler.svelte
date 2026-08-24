<script lang="ts" module>
	export function initFlowGraphAssetsCtx({
		getModules
	}: {
		getModules: () => FlowModule[]
	}): FlowGraphAssetContext {
		let s = $state({
			val: {
				selectedAsset: undefined,
				workspace: undefined,
				s3FilePicker: undefined,
				resourceEditorDrawer: undefined,
				resourceMetadataCache: {},
				additionalAssetsMap: {},
				computeAssetsCount: (asset) => {
					return getAllModules(getModules())
						.flatMap((m) => getFlowModuleAssets(m, s.val.additionalAssetsMap) ?? [])
						.filter((a) => assetEq(asset, a)).length
				},
				sqlQueries: {}
			}
		} satisfies FlowGraphAssetContext)
		return s
	}
</script>

<script lang="ts">
	import { inferAssets } from '$lib/infer'
	import {
		assetEq,
		getFlowModuleAssets,
		type AssetWithAccessType,
		type AssetWithAltAccessType
	} from '../assets/lib'
	import { getAllModules } from './flowExplorer'
	import { getContext } from 'svelte'
	import type { FlowEditorContext, FlowGraphAssetContext } from './types'
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
	import ResourceEditorDrawer from '../ResourceEditorDrawer.svelte'
	import { watch } from 'runed'
	import { sendUserToast } from '$lib/toast'

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
	const { selectionManager, opWorkspace } = getContext<FlowEditorContext>('FlowEditorContext') || {}
	let opWs = $derived(opWorkspace?.() ?? $workspaceStore)
	// Expose the acting workspace to the asset explore controls (ExploreAssetButton
	// reads it from this context; the DB manager / S3 picker act on it).
	$effect(() => {
		if (flowGraphAssetsCtx) flowGraphAssetsCtx.val.workspace = opWs
	})
	let selectedId = $derived(selectionManager?.getSelectedId())

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
			if (asset.kind == 'resource') {
				let truncatedPath = asset.path.split('?table=')[0]
				if (truncatedPath in resMetadataCache) continue
				resMetadataCache[truncatedPath] = undefined // avoid fetching multiple times because of async
				ResourceService.getResource({ path: truncatedPath, workspace: opWs! })
					.then((r) => (resMetadataCache[truncatedPath] = { resource_type: r.resource_type }))
					.catch((err) => console.error("Couldn't fetch resource", truncatedPath, err))
			}
		}
	})

	// Fetch transitive assets (path scripts and flows)
	$effect(() => {
		if (!opWs || !flowGraphAssetsCtx || !enablePathScriptAndFlowAssets) return
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
				workspace: opWs,
				requestBody: { usages }
			}).then((result) => {
				result.forEach((assets, idx) => {
					flowGraphAssetsCtx.val.additionalAssetsMap[modIds[idx]] = assets
				})
			})
		}
	})

	// Ids the flow was loaded with. Only those modules can carry asset metadata predating
	// the assets feature; anything appearing later was added in this session, so writing
	// its assets adds nothing on top of a change the user has already made. The editor
	// only mounts once the flow is loaded, so reading the prop here is the loaded state.
	const loadedModuleIds = new Set(getAllModules(modules).map((m) => m.id))
	// Analyzing is a flow-wide action: offer it once, then apply that answer to every
	// other module of the flow rather than asking again for each one. A prompt left
	// unanswered stays 'offered' and keeps those modules untouched for the session.
	let flowAnalysis: 'unoffered' | 'offered' | 'accepted' = 'unoffered'
	// Last inference per module, keyed on everything inferAssets depends on. The watchers
	// below are re-created whenever a module is added or removed and the selection watch
	// fires on creation, so a miss here means a re-parse on every structural edit.
	type InferredAssets = Extract<Awaited<ReturnType<typeof inferAssets>>, { status: 'ok' }>
	let analyzed: Record<string, { key: string; result: InferredAssets }> = {}

	// Prune per-module caches from deleted modules
	$effect(() => {
		const modulesSet = new Set(allModules.map((m) => m.id))
		for (const key of Object.keys(analyzed)) {
			if (!modulesSet.has(key)) delete analyzed[key]
		}
		for (const key of [...loadedModuleIds]) {
			if (!modulesSet.has(key)) loadedModuleIds.delete(key)
		}
		if (!flowGraphAssetsCtx) return
		for (const key of Object.keys(flowGraphAssetsCtx.val.additionalAssetsMap)) {
			if (!modulesSet.has(key)) {
				delete flowGraphAssetsCtx.val.additionalAssetsMap[key]
			}
		}
	})

	function analyzeEntireFlow() {
		flowAnalysis = 'accepted'
		for (const mod of allModules) {
			if (mod.value.type === 'rawscript') {
				parseAndUpdateRawScriptModule(mod.value, mod.id)
			}
		}
	}

	async function parseAndUpdateRawScriptModule(v: RawScript, modId: string, prompt = false) {
		const key = JSON.stringify([v.language, v.content])
		let inferred = analyzed[modId]?.key === key ? analyzed[modId].result : undefined
		if (!inferred) {
			const inferAssetsResult = await inferAssets(v.language, v.content)
			if (inferAssetsResult.status === 'error') return
			inferred = inferAssetsResult
			analyzed[modId] = { key, result: inferred }
		}
		// Copy before handing anything to the flow store: stored values become reactive
		// proxies, and a later replay of this same inference would mutate the cache.
		const { assets, sql_queries } = structuredClone(inferred)
		if (flowGraphAssetsCtx) flowGraphAssetsCtx.val.sqlQueries[modId] = sql_queries
		let newAssets = assets as AssetWithAltAccessType[]
		for (const asset of newAssets) {
			const old = v.assets?.find((a) => assetEq(a, asset))
			if (old?.alt_access_type) asset.alt_access_type = old.alt_access_type
		}
		const normalizedAssets = newAssets.length > 0 ? newAssets : undefined
		if (!deepEqual(v.assets, normalizedAssets)) {
			if (prompt && flowAnalysis !== 'accepted' && normalizedAssets?.length) {
				if (flowAnalysis === 'offered') return
				flowAnalysis = 'offered'
				// Long-lived because it is the only entry point to analyzeEntireFlow and it is
				// offered once: a toast the user misses cannot be brought back without a reload.
				sendUserToast(
					'Assets were detected in this step. Analyze entire flow for assets?',
					'warning',
					[{ label: 'Analyze entire flow', callback: () => analyzeEntireFlow() }],
					undefined,
					20000
				)
			} else {
				v.assets = normalizedAssets
			}
		}
	}

	$effect(() => {
		if (!enableParser) return
		for (const mod of allModules) {
			const modValue = mod.value
			if (modValue.type === 'rawscript') {
				// Recompute any raw script module assets when its content changes
				watch(
					[() => modValue.content],
					() => {
						parseAndUpdateRawScriptModule(modValue, mod.id)
					},
					{ lazy: true }
				)

				// Also recompute if the module is selected
				watch([() => selectedId === mod.id], () => {
					if (selectedId === mod.id)
						parseAndUpdateRawScriptModule(
							modValue,
							mod.id,
							modValue.assets === undefined && loadedModuleIds.has(mod.id)
						)
				})
			}
		}
	})
</script>

{#if flowGraphAssetsCtx}
	<S3FilePicker bind:this={flowGraphAssetsCtx.val.s3FilePicker} workspace={opWs} readOnlyMode />
	<ResourceEditorDrawer bind:this={flowGraphAssetsCtx.val.resourceEditorDrawer} workspace={opWs} />
{/if}
