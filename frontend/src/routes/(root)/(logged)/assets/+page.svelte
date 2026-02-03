<script lang="ts">
	import { formatAsset, formatAssetKind } from '$lib/components/assets/lib'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import DbManagerDrawer from '$lib/components/DBManagerDrawer.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import S3FilePicker from '$lib/components/S3FilePicker.svelte'
	import { Cell, DataTable } from '$lib/components/table'
	import Head from '$lib/components/table/Head.svelte'
	import {
		AssetService,
		SettingService,
		WorkspaceService,
		type AssetKind,
		type ListAssetsResponse
	} from '$lib/gen'
	import { userStore, workspaceStore, userWorkspaces } from '$lib/stores'
	import { pluralize, truncate } from '$lib/utils'
	import ExploreAssetButton, {
		assetCanBeExplored
	} from '../../../../lib/components/ExploreAssetButton.svelte'
	import AssetsUsageDrawer from '$lib/components/assets/AssetsUsageDrawer.svelte'
	import AssetGenericIcon from '$lib/components/icons/AssetGenericIcon.svelte'
	import { Tooltip } from '$lib/components/meltComponents'
	import { AlertTriangle, Loader2, SettingsIcon } from 'lucide-svelte'
	import { StaleWhileLoading, useInfiniteQuery, useScrollToBottom } from '$lib/svelte5Utils.svelte'
	import { Debounced, resource, watch, type ResourceReturn } from 'runed'
	import Label from '$lib/components/Label.svelte'
	import MultiSelect from '$lib/components/select/MultiSelect.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import RefreshButton from '$lib/components/common/button/RefreshButton.svelte'
	import Section from '$lib/components/Section.svelte'
	import Button from '$lib/components/common/button/Button.svelte'

	interface AssetCursor {
		created_at?: string
		id?: number
	}

	let assetPathFilter: string = $state('')
	let usagePathFilter: string = $state('')
	let assetKindsFilter: Array<AssetKind> = $state([])

	let filters = new Debounced(
		() => ({
			assetPath: assetPathFilter || undefined,
			usagePath: usagePathFilter || undefined,
			assetKinds: assetKindsFilter.join(',') || undefined
		}),
		500
	)

	const assetsQuery = useInfiniteQuery<ListAssetsResponse, AssetCursor | undefined>({
		queryFn: async (cursor) => {
			return AssetService.listAssets({
				workspace: $workspaceStore ?? '',
				perPage: 50,
				cursorCreatedAt: cursor?.created_at,
				cursorId: cursor?.id,
				...filters.current
			})
		},
		initialPageParam: undefined,
		getNextPageParam: (lastPage) => lastPage.next_cursor ?? undefined
	})
	const isAtBottom = useScrollToBottom('#scrollable-container', 250)
	watch(
		() => isAtBottom.current,
		(atBottom) => {
			if (atBottom && assetsQuery.hasNextPage && !assetsQuery.isFetchingNextPage)
				assetsQuery.fetchNextPage()
		}
	)

	watch(
		() => [filters.current, $workspaceStore],
		() => assetsQuery.reset()
	)

	let _assets = new StaleWhileLoading(() =>
		assetsQuery.isLoading ? undefined : assetsQuery.current
	)
	let assets = $derived(_assets.current?.flatMap((page) => page.assets))

	let s3FilePicker: S3FilePicker | undefined = $state()
	let dbManagerDrawer: DbManagerDrawer | undefined = $state()
	let assetsUsageDropdown: AssetsUsageDrawer | undefined = $state()

	let allS3Storages = resource(
		() => $workspaceStore,
		() =>
			SettingService.getSecondaryStorageNames({
				workspace: $workspaceStore!,
				includeDefault: true
			}).then((s) => s.map((s) => (s == '_default_' ? 'Primary storage' : s)))
	)
	let allDucklakes = resource(
		() => $workspaceStore,
		() =>
			WorkspaceService.listDucklakes({ workspace: $workspaceStore! }).then((d) =>
				d.map((d) => (d == 'main' ? 'Primary ducklake' : d))
			)
	)
	let allDataTables = resource(
		() => $workspaceStore,
		() =>
			WorkspaceService.listDataTables({ workspace: $workspaceStore! }).then((d) =>
				d.map((d) => (d == 'main' ? 'Primary data table' : d))
			)
	)
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

		<Section label="All workspace assets">
			<div class="mb-2 pb-8 flex gap-4">
				{#snippet card(props: {
					title: string
					assetKind: AssetKind
					data: ResourceReturn<string[]>
					settingsHref: string
				})}
					<div
						class="flex flex-col bg-surface-tertiary drop-shadow-base rounded-md px-6 py-5 flex-1"
					>
						<div class="divide-y">
							<div class="flex justify-between">
								<h3 class="text-sm font-bold mb-4">{props.title}</h3>
								<a class="text-xs" href={props.settingsHref}>Settings </a>
							</div>
							{#each props.data.current ?? [] as item}
								<div class="text-xs py-2.5 text-primary flex justify-between items-center">
									{item}
									<ExploreAssetButton
										asset={{ kind: props.assetKind, path: item }}
										{s3FilePicker}
										{dbManagerDrawer}
									/>
								</div>
							{/each}
						</div>
						{#if props.data.loading}
							<div class="flex items-center gap-2 mt-2 text-sm text-secondary">
								<Loader2 size={16} class="animate-spin" />
							</div>
						{:else if props.data.error}
							<div class="text-sm text-red-600 mt-2">Error loading {props.title.toLowerCase()}</div>
						{:else if props.data.current?.length === 0}
							<div class="text-sm text-secondary mt-2 mb-3">No {props.title.toLowerCase()} yet</div>
							<Button
								endIcon={{ icon: SettingsIcon }}
								href={props.settingsHref}
								variant="accent"
								wrapperClasses="w-fit"
							>
								{props.title} settings
							</Button>
						{/if}
					</div>
				{/snippet}
				{@render card({
					title: 'Data tables',
					data: allDataTables,
					assetKind: 'datatable',
					settingsHref: '/workspace_settings?tab=windmill_data_tables'
				})}
				{@render card({
					title: 'Ducklakes',
					data: allDucklakes,
					assetKind: 'ducklake',
					settingsHref: '/workspace_settings?tab=windmill_lfs'
				})}
				{@render card({
					title: 'Workspace object storages',
					data: allS3Storages,
					assetKind: 's3object',
					settingsHref: '/workspace_settings?tab=windmill_lfs'
				})}
			</div>
		</Section>
		<Section label="Latest assets used">
			<div class="flex gap-2 mb-4 items-end justify-between">
				<div class="flex gap-2">
					<Label class="lg:min-w-[16rem] max-w-[30rem]" label="Asset path">
						<TextInput bind:value={assetPathFilter} />
					</Label>
					<Label class="lg:min-w-[16rem] max-w-[30rem]" label="Usage path">
						<TextInput bind:value={usagePathFilter} />
					</Label>
					<Label class="lg:min-w-[8rem] max-w-[30rem]" label="Asset kinds">
						<MultiSelect
							hideMainClearBtn
							bind:value={assetKindsFilter}
							items={[
								{ label: 'S3 Object', value: 's3object' },
								{ label: 'Ducklake', value: 'ducklake' },
								{ label: 'Data Table', value: 'datatable' }
							]}
						/>
					</Label>
				</div>
				<RefreshButton onClick={() => assetsQuery.reset()} loading={assetsQuery.isLoading} />
			</div>
			{@render table()}
			{#if assetsQuery.isFetchingNextPage}
				<Loader2 size={32} class="mx-auto my-4 text-primary animate-spin" />
			{:else if assets?.length && !assetsQuery.hasNextPage}
				<div class="text-center text-2xs text-secondary my-4"> No more assets to load </div>
			{/if}
		</Section>
	</CenteredPage>
{/if}

<AssetsUsageDrawer bind:this={assetsUsageDropdown} />
<S3FilePicker bind:this={s3FilePicker} readOnlyMode />
<DbManagerDrawer bind:this={dbManagerDrawer} />

{#snippet table()}
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
			{#if assets != undefined && assets.length === 0}
				<tr class="h-14">
					<Cell></Cell>
					<Cell colspan="3">No assets found</Cell>
				</tr>
			{/if}
			{#each assets as asset}
				{@const assetUri = formatAsset(asset)}
				<tr class="h-12">
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
			{#if assets == undefined && assetsQuery.isLoading}
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
{/snippet}
