<script lang="ts">
	import { formatAsset, getAssetUsagePageUri } from '$lib/components/assets/lib'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { ClearableInput, DrawerContent, Tab, Tabs } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'
	import DbManagerDrawer from '$lib/components/DBManagerDrawer.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import S3FilePicker from '$lib/components/S3FilePicker.svelte'
	import { Cell, DataTable } from '$lib/components/table'
	import Head from '$lib/components/table/Head.svelte'
	import { AssetService, ResourceService, type ListAssetsResponse } from '$lib/gen'
	import { userStore, workspaceStore, userWorkspaces } from '$lib/stores'
	import { usePromise } from '$lib/svelte5Utils.svelte'
	import { pluralize, truncate } from '$lib/utils'
	import { untrack } from 'svelte'
	import ExploreAssetButton, { assetCanBeExplored } from './ExploreAssetButton.svelte'
	import {
		assetDisplaysAsInputInFlowGraph,
		assetDisplaysAsOutputInFlowGraph
	} from '$lib/components/graph/renderers/nodes/AssetNode.svelte'
	import { Boxes, DollarSign, Pyramid } from 'lucide-svelte'
	import AssetGenericIcon from '$lib/components/icons/AssetGenericIcon.svelte'
	import S3Icon from '$lib/components/icons/S3Icon.svelte'

	let assets = usePromise(() => AssetService.listAssets({ workspace: $workspaceStore ?? '' }))

	let filterText: string = $state('')
	let filteredAssets = $derived(
		assets.value?.filter((asset) =>
			formatAsset(asset).toLowerCase().includes(filterText.toLowerCase())
		) ?? []
	)

	let resourceTypesCache: Record<string, string | undefined> = $state({})
	$effect(() => {
		if (!assets.value) return
		untrack(() => {
			for (const asset of assets.value) {
				if (asset.kind !== 'resource' || asset.path in resourceTypesCache) continue
				ResourceService.getResource({ path: asset.path, workspace: $workspaceStore! })
					.then((resource) => (resourceTypesCache[asset.path] = resource.resource_type))
					.catch((err) => (resourceTypesCache[asset.path] = undefined))
			}
		})
	})

	let viewOccurences: ListAssetsResponse[number] | undefined = $state()
	let s3FilePicker: S3FilePicker | undefined = $state()
	let dbManagerDrawer: DbManagerDrawer | undefined = $state()

	let selectedTab: 'workspace' | 'resources' | 'variables' | 's3objects' = $state('workspace')
</script>

{#if $userStore?.operator && $workspaceStore && !$userWorkspaces.find((_) => _.id === $workspaceStore)?.operator_settings?.resources}
	<div class="bg-red-100 border-l-4 border-red-600 text-orange-700 p-4 m-4 mt-12" role="alert">
		<p class="font-bold">Unauthorized</p>
		<p>Page not available for operators</p>
	</div>
{:else}
	<CenteredPage>
		<PageHeader
			title="Assets"
			tooltip="Manage your assets and see where they are used in the workspace."
			documentationLink="https://www.windmill.dev/docs/core_concepts/assets"
		/>
		<Tabs bind:selected={selectedTab}>
			<Tab size="md" value="workspace">
				<div class="flex gap-2 items-center my-1">
					<Pyramid size={18} />
					All assets
				</div>
			</Tab>
			<Tab size="md" value="resources">
				<div class="flex gap-2 items-center my-1">
					<Boxes size={18} />
					Resources
				</div>
			</Tab>
			<Tab size="md" value="variables">
				<div class="flex gap-2 items-center my-1">
					<DollarSign size={18} />
					Variables
				</div>
			</Tab>
			<Tab size="md" value="s3objects">
				<div class="flex gap-2 items-center my-1">
					<S3Icon />
					S3 Objects
				</div>
			</Tab>
		</Tabs>
		<ClearableInput bind:value={filterText} placeholder="Search assets" class="my-4" />
		<DataTable>
			<Head>
				<tr>
					<Cell head first></Cell>
					<Cell head>Asset name</Cell>
					<Cell head></Cell>
					<Cell head></Cell>
				</tr>
			</Head>
			<tbody class="divide-y bg-surface">
				{#if filteredAssets.length === 0}
					<tr class="h-14">
						<Cell colspan="3" class="text-center text-tertiary">No assets found</Cell>
					</tr>
				{/if}
				{#each filteredAssets as asset}
					{@const assetUri = formatAsset(asset)}
					<tr class="h-14">
						<Cell first>
							<AssetGenericIcon
								assetKind={asset.kind}
								fill=""
								class="fill-secondary stroke-secondary"
							/>
						</Cell>
						<Cell class="w-[75%]">{truncate(asset.path, 92)}</Cell>
						<Cell>
							<a href={`#${assetUri}`} onclick={() => (viewOccurences = asset)}>
								{pluralize(asset.usages.length, 'occurrence')}
							</a>
						</Cell>
						<Cell>
							{#if assetCanBeExplored(asset, asset.metadata)}
								<ExploreAssetButton
									{asset}
									{s3FilePicker}
									{dbManagerDrawer}
									_resourceMetadata={{ resource_type: resourceTypesCache[asset.path] }}
									class="w-24"
								/>
							{/if}
						</Cell>
					</tr>
				{/each}
			</tbody>
		</DataTable>
	</CenteredPage>
{/if}

<Drawer
	open={viewOccurences !== undefined}
	size="900px"
	on:close={() => (viewOccurences = undefined)}
>
	<DrawerContent title="Asset occurrences" on:close={() => (viewOccurences = undefined)}>
		<ul class="flex flex-col border rounded-md divide-y">
			{#each viewOccurences?.usages ?? [] as u}
				<li>
					<a
						href={getAssetUsagePageUri(u)}
						aria-label={`${u.kind}/${u.path}`}
						class="text-sm text-primary flex items-center py-3 px-4 gap-3 hover:bg-surface-hover cursor-pointer"
					>
						<RowIcon kind={u.kind} />
						<div class="flex flex-col justify-center flex-1">
							<span class="font-semibold">{u.path}</span>
							<span class="text-xs text-tertiary">{u.kind}</span>
						</div>
						<div class="flex gap-2">
							{#if assetDisplaysAsInputInFlowGraph(u)}
								<div class="text-xs border text-tertiary max-w-fit p-1 rounded-md">Read</div>
							{/if}
							{#if assetDisplaysAsOutputInFlowGraph(u)}
								<div class="text-xs border text-tertiary max-w-fit p-1 rounded-md">Write</div>
							{/if}
						</div>
					</a>
				</li>
			{/each}
		</ul>
	</DrawerContent>
</Drawer>
<S3FilePicker bind:this={s3FilePicker} readOnlyMode />
<DbManagerDrawer bind:this={dbManagerDrawer} />
