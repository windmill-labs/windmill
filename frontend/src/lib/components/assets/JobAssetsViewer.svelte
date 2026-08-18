<script lang="ts">
	import { JobService, ResourceService, ScriptService, type Job } from '$lib/gen'
	import { inferAssets } from '$lib/infer'
	import { workspaceStore } from '$lib/stores'
	import { usePromise } from '$lib/svelte5Utils.svelte'
	import { pruneNullishArray, uniqueBy } from '$lib/utils'
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

	async function extractAssets(job: Job): Promise<AssetWithAccessType[]> {
		const [runtimeAssets, staticAssets] = await Promise.all([
			fetchRuntimeAssets(job),
			inferStaticAssets(job)
		])
		// Runtime assets first: they carry the access type the run actually had,
		// and a static match for the same asset carries nothing extra.
		return uniqueBy([...runtimeAssets, ...staticAssets], (x) => x.kind + x.path)
	}

	// What the run touched, as recorded by runtime detection. Static inference
	// misses these whenever the path is only known at runtime, and it never sees
	// what a flow step or a workflow-as-code task did on the parent's behalf.
	async function fetchRuntimeAssets(job: Job): Promise<AssetWithAccessType[]> {
		if (!$workspaceStore) return []
		return await JobService.listRunAssets({
			workspace: $workspaceStore,
			id: job.id
		}).catch((err) => {
			console.error("Couldn't fetch runtime assets of job", job.id, err)
			return []
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
		for (const asset of assets.value ?? []) {
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

{#if assets.value && assets.value.length > 0}
	<ul class="flex flex-col divide-y mt-1">
		{#each assets.value ?? [] as asset}
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
				<AssetButtons
					{asset}
					{resourceDataCache}
					{resourceEditorDrawer}
					{s3FilePicker}
				/>
			</li>
		{/each}
	</ul>
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
