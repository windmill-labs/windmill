<script lang="ts">
	import { formatAsset, formatAssetKind } from '$lib/components/assets/lib'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { Alert, ClearableInput } from '$lib/components/common'
	import DbManagerDrawer from '$lib/components/DBManagerDrawer.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import S3FilePicker from '$lib/components/S3FilePicker.svelte'
	import { Cell, DataTable } from '$lib/components/table'
	import Head from '$lib/components/table/Head.svelte'
	import { AssetService, type ListAssetsResponse } from '$lib/gen'
	import { userStore, workspaceStore, userWorkspaces } from '$lib/stores'
	import { pluralize, truncate } from '$lib/utils'
	import ExploreAssetButton, {
		assetCanBeExplored
	} from '../../../../lib/components/ExploreAssetButton.svelte'
	import AssetsUsageDrawer from '$lib/components/assets/AssetsUsageDrawer.svelte'
	import AssetGenericIcon from '$lib/components/icons/AssetGenericIcon.svelte'
	import { Tooltip } from '$lib/components/meltComponents'
	import { AlertTriangle, Loader2 } from 'lucide-svelte'
	import { useInfiniteQuery, useScrollToBottom } from '$lib/svelte5Utils.svelte'
	import { watch } from 'runed'

	interface AssetCursor {
		created_at?: string
		id?: number
	}

	const assetsQuery = useInfiniteQuery<ListAssetsResponse, AssetCursor | undefined>({
		queryFn: async (cursor) => {
			return await AssetService.listAssets({
				workspace: $workspaceStore ?? '',
				perPage: 50,
				cursorCreatedAt: cursor?.created_at,
				cursorId: cursor?.id
			})
		},
		initialPageParam: undefined,
		getNextPageParam: (lastPage) => lastPage.next_cursor ?? undefined
	})
	const isAtBottom = useScrollToBottom('#scrollable-container', 250)
	watch(
		() => isAtBottom.current,
		(atBottom) => {
			if (atBottom && assetsQuery.hasNextPage && !assetsQuery.isFetchingNextPage) {
				assetsQuery.fetchNextPage()
			}
		}
	)

	// Reset query when workspace changes
	watch(
		() => $workspaceStore,
		() => {
			assetsQuery.reset()
		}
	)

	let filterText: string = $state('')
	let filteredAssets = $derived(
		assetsQuery.current
			.flatMap((page) => page.assets)
			.filter((asset) => formatAsset(asset).toLowerCase().includes(filterText.toLowerCase())) ?? []
	)

	let s3FilePicker: S3FilePicker | undefined = $state()
	let dbManagerDrawer: DbManagerDrawer | undefined = $state()
	let assetsUsageDropdown: AssetsUsageDrawer | undefined = $state()
</script>

{#if $userStore?.operator && $workspaceStore && !$userWorkspaces.find((_) => _.id === $workspaceStore)?.operator_settings?.assets}
	<div class="bg-red-100 border-l-4 border-red-600 text-orange-700 p-4 m-4 mt-12" role="alert">
		<p class="font-bold">Unauthorized</p>
		<p>Page not available for operators</p>
	</div>
{:else}
	<CenteredPage id="scrollable-container">
		<PageHeader
			title="Assets"
			tooltip="Assets show up here whenever you use them in Windmill."
			documentationLink="https://www.windmill.dev/docs/core_concepts/assets"
		/>
		<Alert title="Assets may be missing from this page" type="info" class="mb-4">
			Assets are not detected for old scripts and flows that were deployed before the assets feature
			was introduced. Re-deploy them to trigger asset detection.
		</Alert>
		<ClearableInput bind:value={filterText} placeholder="Search assets" class="mb-4" />
		<DataTable>
			<Head>
				<tr>
					<Cell head first class="w-16"></Cell>
					<Cell head>Asset name</Cell>
					<Cell head></Cell>
					<Cell head></Cell>
				</tr>
			</Head>
			<tbody class="divide-y bg-surface">
				{#if !assetsQuery.error && !assetsQuery.isLoading && filteredAssets.length === 0}
					<tr class="h-14">
						<Cell colspan="4">No assets found</Cell>
					</tr>
				{/if}
				{#each filteredAssets as asset}
					{@const assetUri = formatAsset(asset)}
					<tr class="h-14">
						<Cell first class="w-16">
							<Tooltip>
								<AssetGenericIcon assetKind={asset.kind} size="16px" class="!text-secondary" />
								<svelte:fragment slot="text">{formatAssetKind(asset)}</svelte:fragment>
							</Tooltip>
						</Cell>
						<Cell class="flex flex-col">
							<span>{truncate(asset.path, 92)}</span>
							<span class="text-2xs text-secondary">{formatAssetKind(asset)}</span>
						</Cell>
						<Cell>
							<a href={`#${assetUri}`} onclick={() => assetsUsageDropdown?.open(asset)}>
								{pluralize(asset.usages.length, 'usage')}
							</a>
						</Cell>
						<Cell class="w-24">
							{#if assetCanBeExplored(asset, asset.metadata) && !$userStore?.operator}
								<ExploreAssetButton
									{asset}
									{s3FilePicker}
									{dbManagerDrawer}
									_resourceMetadata={asset.metadata}
								/>
							{/if}
							{#if asset.kind === 'resource' && asset.metadata === undefined}
								<Tooltip class={'w-24 flex items-center justify-center'}>
									<AlertTriangle size={20} class="text-orange-600 dark:text-orange-500" />
									<svelte:fragment slot="text">Could not find resource</svelte:fragment>
								</Tooltip>
							{/if}
						</Cell>
					</tr>
				{/each}
				{#if assetsQuery.isLoading}
					<tr class="h-14">
						<Cell colspan="4" class="text-center">
							<div class="flex items-center justify-center gap-2">
								<Loader2 class="animate-spin" size={16} />
								<span>Loading assets...</span>
							</div>
						</Cell>
					</tr>
				{/if}
			</tbody>
		</DataTable>
		{#if assetsQuery.isLoading}
			<Loader2 size={32} class="mx-auto my-4 text-primary animate-spin" />
		{:else if !assetsQuery.hasNextPage}
			<div class="text-center text-2xs text-secondary my-4"> No more assets to load </div>
		{/if}
	</CenteredPage>
{/if}

<AssetsUsageDrawer bind:this={assetsUsageDropdown} />
<S3FilePicker bind:this={s3FilePicker} readOnlyMode />
<DbManagerDrawer bind:this={dbManagerDrawer} />
