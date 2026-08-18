<script lang="ts">
	import { JobService, ResourceService, ScriptService, type Job } from '$lib/gen'
	import { inferAssets } from '$lib/infer'
	import { workspaceStore } from '$lib/stores'
	import { usePromise } from '$lib/svelte5Utils.svelte'
	import { pruneNullishArray, uniqueBy } from '$lib/utils'
	import { Skeleton } from '../common'
	import ResourceEditorDrawer from '../ResourceEditorDrawer.svelte'
	import S3FilePicker from '../S3FilePicker.svelte'
	import AssetButtons from './AssetButtons.svelte'
	import {
		formatAssetAccessType,
		formatAssetKind,
		getFlowModuleAssets,
		parseInputArgsAssets,
		type AssetWithAccessType
	} from './lib'

	type Props = {
		job: Job
	}
	let { job }: Props = $props()

	const assetKey = (a: AssetWithAccessType) => a.kind + a.path

	async function extractAssets(
		job: Job
	): Promise<{ assets: AssetWithAccessType[]; truncated: boolean }> {
		const [runtime, staticAssets] = await Promise.all([
			fetchRuntimeAssets(job),
			inferStaticAssets(job)
		])
		// Runtime assets win: they are what the run actually did. Their access type
		// can still be unknown — a resource passed in the arguments is recorded
		// without one — so fall back to what the parser inferred for the same asset.
		// One asset can appear twice statically, parsed from the code and again from
		// the arguments; only the parsed one carries an access type.
		const staticByKey = new Map<string, AssetWithAccessType>()
		for (const a of staticAssets) {
			const prev = staticByKey.get(assetKey(a))
			if (!prev || (!prev.access_type && a.access_type)) staticByKey.set(assetKey(a), a)
		}
		const merged = runtime.assets.map((a) => ({
			...a,
			access_type: a.access_type ?? staticByKey.get(assetKey(a))?.access_type
		}))
		return {
			assets: uniqueBy([...merged, ...staticAssets], assetKey),
			truncated: runtime.truncated
		}
	}

	// What the run touched, as recorded by runtime detection. Static inference
	// misses these whenever the path is only known at runtime, and it never sees
	// what a flow step or a workflow-as-code task did on the parent's behalf.
	async function fetchRuntimeAssets(
		job: Job
	): Promise<{ assets: AssetWithAccessType[]; truncated: boolean }> {
		if (!$workspaceStore) return { assets: [], truncated: false }
		return await JobService.listRunAssets({
			workspace: $workspaceStore,
			id: job.id
		}).catch((err) => {
			console.error("Couldn't fetch runtime assets of job", job.id, err)
			return { assets: [], truncated: false }
		})
	}

	async function inferStaticAssets(job: Job): Promise<AssetWithAccessType[]> {
		if (job.job_kind === 'flow') {
			const additionalAssetsMap = {
				// TODO : Transitive assets
			}
			return uniqueBy(
				pruneNullishArray([
					...(job.raw_flow?.modules.flatMap((m) => getFlowModuleAssets(m, additionalAssetsMap)) ??
						[]),
					...parseInputArgsAssets(job.args ?? {})
				]),
				(x) => x.kind + x.path
			)
		}

		if (job.job_kind === 'script') {
			let code = job.raw_code
			if (!code && job.script_hash && $workspaceStore) {
				const script = await ScriptService.getScriptByHash({
					workspace: $workspaceStore,
					hash: job.script_hash
				})
				code = script.content
			}
			let inferAssetsResult = await inferAssets(job.language!, code ?? '')
			let assets = inferAssetsResult.status === 'ok' ? inferAssetsResult.assets : []
			return [...assets, ...parseInputArgsAssets(job.args ?? {})]
		}
		return []
	}

	let assets = usePromise(() => extractAssets(job), { loadInit: false })
	$effect(() => {
		job.id
		$workspaceStore
		assets.refresh()
	})

	let resourceDataCache: Record<string, string | undefined> = $state({})
	$effect(() => {
		for (const asset of assets.value?.assets ?? []) {
			if (asset.kind == 'resource') {
				let truncatedPath = asset.path.split('?table=')[0]
				if (truncatedPath in resourceDataCache) continue
				resourceDataCache[truncatedPath] = undefined // avoid fetching multiple times because of async
				ResourceService.getResource({ path: truncatedPath, workspace: $workspaceStore! })
					.then((r) => (resourceDataCache[truncatedPath] = r.resource_type))
					.catch((err) => console.error("Couldn't fetch resource", truncatedPath, err))
			}
		}
	})

	let s3FilePicker: S3FilePicker | undefined = $state()
	let resourceEditorDrawer: ResourceEditorDrawer | undefined = $state()
</script>

{#if assets.status === 'idle' || assets.status === 'loading'}
	<Skeleton layout={[[3], 0.5, [3]]} class="w-full" />
{:else if assets.value && assets.value.assets.length > 0}
	<ul class="flex flex-col divide-y mt-1">
		{#each assets.value.assets as asset}
			<li class="flex justify-between items-center gap-2 py-3 leading-4 text-sm pl-4">
				<div class="flex flex-col flex-1 truncate">
					{asset.path}
					<span class="text-2xs text-primary">
						{formatAssetKind({
							...asset,
							...(asset.kind === 'resource'
								? { metadata: { resource_type: resourceDataCache[asset.path.split('?table=')[0]] } }
								: {})
						})}
					</span>
				</div>
				{#if asset.access_type}
					<span class="text-xs text-secondary">{formatAssetAccessType(asset.access_type)}</span>
				{/if}
				<AssetButtons {asset} {resourceDataCache} {resourceEditorDrawer} {s3FilePicker} />
			</li>
		{/each}
	</ul>
	{#if assets.value.truncated}
		<div class="text-2xs text-secondary mt-2 pl-4">
			This run touched more assets than are listed here.
		</div>
	{/if}
{:else}
	<div class="flex flex-col gap-1">
		<span class="text-sm text-primary">No assets found</span>
		<span class="text-2xs text-secondary">
			Assets detected while a run executes are recorded asynchronously, and only the most recent
			runs that touched an asset keep that record.
		</span>
	</div>
{/if}

<S3FilePicker bind:this={s3FilePicker} readOnlyMode />
<ResourceEditorDrawer bind:this={resourceEditorDrawer} />
