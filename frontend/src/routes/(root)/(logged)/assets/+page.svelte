<script lang="ts">
	import { formatAsset, formatAssetKind } from '$lib/components/assets/lib'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
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
	import { userStore, workspaceStore, userWorkspaces, globalDbManagerDrawer } from '$lib/stores'
	import { parseDbInputFromAssetSyntax, pluralize, truncate } from '$lib/utils'
	import ExploreAssetButton, {
		assetCanBeExplored
	} from '../../../../lib/components/ExploreAssetButton.svelte'
	import AssetsUsageDrawer from '$lib/components/assets/AssetsUsageDrawer.svelte'
	import AssetGenericIcon from '$lib/components/icons/AssetGenericIcon.svelte'
	import { Tooltip } from '$lib/components/meltComponents'
	import { AlertTriangle, Loader2, SettingsIcon, StarIcon } from 'lucide-svelte'
	import { StaleWhileLoading, useInfiniteQuery, useScrollToBottom } from '$lib/svelte5Utils.svelte'
	import { Debounced, resource, watch, type ResourceReturn } from 'runed'
	import Label from '$lib/components/Label.svelte'
	import MultiSelect from '$lib/components/select/MultiSelect.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import RefreshButton from '$lib/components/common/button/RefreshButton.svelte'
	import Section from '$lib/components/Section.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { favoriteManager, parseFavoriteAsset } from '$lib/components/sidebar/FavoriteMenu.svelte'

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
	let dbManagerDrawer = $derived(globalDbManagerDrawer.val) as any
	let assetsUsageDropdown: AssetsUsageDrawer | undefined = $state()

	let allS3Storages = resource(
		() => $workspaceStore,
		() =>
			SettingService.getSecondaryStorageNames({
				workspace: $workspaceStore!,
				includeDefault: true
			}).then((s) =>
				s.map((s) =>
					s == '_default_' ? { label: 'Primary storage', value: '/' } : { label: s, value: `${s}/` }
				)
			)
	)
	let allDucklakes = resource(
		() => $workspaceStore,
		() =>
			WorkspaceService.listDucklakes({ workspace: $workspaceStore! }).then((d) =>
				d.map((d) => ({ label: d == 'main' ? 'Main ducklake' : d, value: d }))
			)
	)
	let allDataTables = resource(
		() => $workspaceStore,
		() =>
			WorkspaceService.listDataTables({ workspace: $workspaceStore! }).then((d) =>
				d.map((d) => ({ label: d == 'main' ? 'Main data table' : d, value: d }))
			)
	)

	function extractFavorites(kind: AssetKind) {
		return favoriteManager.current
			.filter((f) => f.kind === 'asset' && f.path.startsWith(kind))
			.map((f) => parseFavoriteAsset(f.path))
	}
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

		<Section label="All workspace assets" class="mb-20">
			<div class="flex gap-4">
				{#snippet card(props: {
					title: string
					assetKind: AssetKind
					data: ResourceReturn<{ label: string; value: string }[]>
					settingsHref: string
					docsHref: string
					favorites?: { table: string; schema?: string; assetName: string; path: string }[]
				})}
					<div class="flex flex-col bg-surface-tertiary drop-shadow-base rounded-md flex-1">
						<div class="flex justify-between border-b">
							<h3 class="text-sm font-bold mb-4 pt-5 pl-6">{props.title}</h3>
							<div class="flex items-center h-fit gap-2 mt-4 mr-4">
								<Button
									wrapperClasses="h-fit"
									btnClasses="text-accent"
									variant="subtle"
									unifiedSize="sm"
									href={props.docsHref}
									target="_blank"
								>
									See documentation
								</Button>
								<Button
									wrapperClasses="h-fit"
									variant={props.data.current?.length === 0 && !props.data.loading
										? 'accent'
										: 'subtle'}
									iconOnly
									endIcon={{ icon: SettingsIcon }}
									href={props.settingsHref}
								/>
							</div>
						</div>
						{#if props.data.current?.length}
							<div class="max-h-96 overflow-y-auto pb-1">
								{#each props.data.current ?? [] as item}
									<div class="text-xs py-2 text-primary flex justify-between items-center px-6">
										{item.label}
										<ExploreAssetButton
											asset={{ kind: props.assetKind, path: item.value }}
											{s3FilePicker}
											{dbManagerDrawer}
											btnClasses="dark:bg-surface"
										/>
									</div>
								{/each}
							</div>

							{#if !props.data.loading && !props.data.error && props.favorites != undefined}
								<div class="mb-4 pt-2 px-6">
									<h3 class="text-xs font-bold mb-1"> Favorite tables</h3>
									<div class="flex gap-1 flex-wrap">
										{#each props.favorites as fav}
											<button
												class="text-xs font-normal bg-surface-sunken rounded-md px-2 py-1 cursor-pointer hover:opacity-80"
												onclick={() => {
													const dbInput = parseDbInputFromAssetSyntax(fav.path)
													if (dbInput) globalDbManagerDrawer.val?.openDrawer(dbInput)
												}}
											>
												<span>
													<StarIcon size="12" class="inline mr-1" />
													{fav.schema ? `${fav.schema}.` : ''}{fav.table}
												</span>
												<span class="text-2xs">
													{fav.assetName === 'main' ? `` : `(${fav.assetName})`}
												</span>
											</button>
										{/each}
									</div>
									{#if props.favorites.length === 0}
										<div class="text-xs text-secondary"> No favorite table yet</div>
									{/if}
								</div>
							{/if}
						{/if}
						{#if props.data.loading}
							<div class="flex items-center gap-2 mt-2 mb-5 px-6 text-sm text-secondary">
								<Loader2 size={16} class="animate-spin" />
							</div>
						{:else if props.data.error}
							<div class="text-sm text-red-600 mt-2 mb-5 px-6">
								Error loading {props.title.toLowerCase()}
							</div>
						{:else if props.data.current?.length === 0}
							<div class="text-xs text-secondary mt-2 px-6 mb-3">
								No {props.title.toLowerCase()} yet
							</div>
						{/if}
					</div>
				{/snippet}
				{@render card({
					title: 'Data table',
					data: allDataTables,
					assetKind: 'datatable',
					settingsHref: '/workspace_settings?tab=windmill_data_tables',
					docsHref: 'https://www.windmill.dev/docs/core_concepts/persistent_storage/data_tables',
					favorites: extractFavorites('datatable')
				})}
				{@render card({
					title: 'Ducklake',
					data: allDucklakes,
					assetKind: 'ducklake',
					settingsHref: '/workspace_settings?tab=windmill_lfs',
					docsHref: 'https://www.windmill.dev/docs/core_concepts/persistent_storage/ducklake',
					favorites: extractFavorites('ducklake')
				})}
				{@render card({
					title: 'Object storage',
					data: allS3Storages,
					assetKind: 's3object',
					settingsHref: '/workspace_settings?tab=windmill_lfs',
					docsHref:
						'https://www.windmill.dev/docs/core_concepts/persistent_storage/large_data_files'
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
								{ label: 'Data Table', value: 'datatable' },
								{ label: 'Resource', value: 'resource' }
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
