<script lang="ts">
	import { ResourceService, type Job } from '$lib/gen'
	import { inferAssets } from '$lib/infer'
	import { workspaceStore } from '$lib/stores'
	import { usePromise } from '$lib/svelte5Utils.svelte'
	import { pruneNullishArray, uniqueBy } from '$lib/utils'
	import DbManagerDrawer from '../DBManagerDrawer.svelte'
	import ResourceEditorDrawer from '../ResourceEditorDrawer.svelte'
	import S3FilePicker from '../S3FilePicker.svelte'
	import AssetButtons from './AssetButtons.svelte'
	import {
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
			return [
				...(await inferAssets(job.language!, job.raw_code ?? '')),
				...parseInputArgsAssets(job.args ?? {})
			]
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
			if (asset.kind !== 'resource' || asset.path in resourceDataCache) continue
			ResourceService.getResource({ path: asset.path, workspace: $workspaceStore! })
				.then((resource) => (resourceDataCache[asset.path] = resource.resource_type))
				.catch((err) => (resourceDataCache[asset.path] = undefined))
		}
	})

	let s3FilePicker: S3FilePicker | undefined = $state()
	let dbManagerDrawer: DbManagerDrawer | undefined = $state()
	let resourceEditorDrawer: ResourceEditorDrawer | undefined = $state()
</script>

<ul class="flex flex-col divide-y mt-1">
	{#each assets.value ?? [] as asset}
		<li class="flex justify-between py-3 leading-4 text-sm pl-4">
			<div class="flex flex-col flex-1 truncate">
				{asset.path}
				<span class="text-2xs text-tertiary">
					{formatAssetKind({
						...asset,
						...(asset.kind === 'resource'
							? { metadata: { resource_type: resourceDataCache[asset.path] } }
							: {})
					})}
				</span>
			</div>
			<AssetButtons
				{asset}
				{resourceDataCache}
				{dbManagerDrawer}
				{resourceEditorDrawer}
				{s3FilePicker}
			/>
		</li>
	{/each}
</ul>

<S3FilePicker bind:this={s3FilePicker} readOnlyMode />
<DbManagerDrawer bind:this={dbManagerDrawer} />
<ResourceEditorDrawer bind:this={resourceEditorDrawer} />
