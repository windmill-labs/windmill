<script lang="ts">
	import { formatAsset, formatAssetKind } from '$lib/components/assets/lib'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import S3FilePicker from '$lib/components/S3FilePicker.svelte'
	import { Cell, DataTable } from '$lib/components/table'
	import Head from '$lib/components/table/Head.svelte'
	import {
		AssetService,
		ScriptService,
		SettingService,
		WorkspaceService,
		type AssetKind,
		type ListAssetsResponse
	} from '$lib/gen'
	import {
		userStore,
		workspaceStore,
		userWorkspaces,
		globalDbManagerDrawer,
		superadmin
	} from '$lib/stores'
	import { parseDbInputFromAssetSyntax, pluralize, truncate } from '$lib/utils'
	import ExploreAssetButton, {
		assetCanBeExplored
	} from '../../../../lib/components/ExploreAssetButton.svelte'
	import AssetsUsageDrawer from '$lib/components/assets/AssetsUsageDrawer.svelte'
	import AssetGenericIcon from '$lib/components/icons/AssetGenericIcon.svelte'
	import { Tooltip } from '$lib/components/meltComponents'
	import { AlertTriangle, Loader2, SettingsIcon, StarIcon, TableProperties } from 'lucide-svelte'
	import { previewDbtRows, type DbtPreview } from '$lib/components/dbt/previewRows'
	import { nodeSelector } from '$lib/components/dbt/parseDbtRun'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import { StaleWhileLoading, useInfiniteQuery, useScrollToBottom } from '$lib/svelte5Utils.svelte'
	import { resource, watch, type ResourceReturn } from 'runed'
	import RefreshButton from '$lib/components/common/button/RefreshButton.svelte'
	import Section from '$lib/components/Section.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { favoriteManager, parseFavoriteAsset } from '$lib/components/sidebar/FavoriteMenu.svelte'
	import FilterSearchbar, {
		useUrlSyncedFilterInstance
	} from '$lib/components/FilterSearchbar.svelte'
	import { buildAssetsFilterSchema } from '$lib/components/assets/assetsFilter'
	import { untrack } from 'svelte'
	import { VolumeService } from '$lib/gen'
	import VolumesDrawer from '$lib/components/assets/VolumesDrawer.svelte'
	import { HardDriveIcon } from 'lucide-svelte'

	interface AssetCursor {
		created_at?: string
		id?: number
	}

	// Collect unique values for filter autocomplete
	let allPaths: string[] = $state([])
	let allAssetKinds: string[] = $state([
		's3object',
		'resource',
		'variable',
		'ducklake',
		'datatable',
		'volume',
		'dbt'
	])

	// FilterSearchbar setup
	let assetsFilterSchema = $derived(
		buildAssetsFilterSchema({
			paths: allPaths,
			assetKinds: allAssetKinds
		})
	)
	let filterValues = useUrlSyncedFilterInstance(untrack(() => assetsFilterSchema))

	let filters = $derived.by(() => ({
		assetPath: filterValues.val.asset_path || undefined,
		usagePath: filterValues.val.usage_path || undefined,
		assetKinds: filterValues.val.asset_kinds || undefined,
		path: filterValues.val.path || undefined,
		columns: filterValues.val.columns || undefined,
		broadFilter: filterValues.val._default_ || undefined
	}))

	const assetsQuery = useInfiniteQuery<ListAssetsResponse, AssetCursor | undefined>({
		queryFn: async (cursor) => {
			return AssetService.listAssets({
				workspace: $workspaceStore ?? '',
				perPage: 50,
				cursorCreatedAt: cursor?.created_at,
				cursorId: cursor?.id,
				...filters
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
		() => [filters, $workspaceStore],
		() => assetsQuery.reset()
	)

	let _assets = new StaleWhileLoading(() =>
		assetsQuery.isLoading ? undefined : assetsQuery.current
	)
	let assets = $derived(_assets.current?.flatMap((page) => page.assets))

	let s3FilePicker: S3FilePicker | undefined = $state()
	let assetsUsageDropdown: AssetsUsageDrawer | undefined = $state()

	// A `dbt://` asset's rows, previewed through the dbt project that writes it.
	// The producer is the asset's own WRITE usage — a table nobody writes has no
	// project to ask, and no `ref()` to resolve the relation with.
	type AssetRow = { path: string; kind: string; usages: { path: string; kind: string; access_type?: string }[] }
	function dbtProducerOf(asset: AssetRow): string | undefined {
		if (asset.kind !== 'dbt') return undefined
		// A DEFINITE writer. A bare `dbt://…` literal in a native script is stored
		// with no access type at all, so treating "not read" as "writes" would pick
		// a Python consumer — and previewing would then RUN that script and its side
		// effects instead of `dbt show`. Its being a dbt script is checked before
		// anything is submitted.
		return asset.usages.find(
			(u) => u.kind === 'script' && (u.access_type === 'w' || u.access_type === 'rw')
		)?.path
	}
	let previewDrawer: Drawer | undefined = $state()
	let previewTitle = $state('')
	let preview = $state<DbtPreview | undefined>(undefined)
	let previewSeq = 0
	// A warehouse row can hold JSON, an array or a struct; `String()` renders those
	// as `[object Object]`, which says less than nothing about the data.
	function cell(v: unknown): string {
		if (v == undefined) return ''
		return typeof v === 'object' ? JSON.stringify(v) : String(v)
	}
	async function previewTable(asset: AssetRow) {
		const ws = $workspaceStore
		const script = dbtProducerOf(asset)
		if (!ws || !script) return
		previewTitle = asset.path.split('/').pop() ?? asset.path
		preview = { pending: true }
		previewDrawer?.openDrawer()
		const seq = ++previewSeq
		const fail = (error: string) => {
			if (seq === previewSeq) preview = { error }
		}
		try {
			// A relation outlives the script that wrote it: deleting a dbt script
			// leaves its `usages` behind, and the raw 404 that follows says nothing
			// about which script or why.
			const producer = await ScriptService.getScriptByPath({
				workspace: ws,
				path: script
			}).catch(() => undefined)
			if (!producer) {
				return fail(
					`${script} is recorded as writing this table but no longer exists, so there is ` +
						`nothing to preview it with.`
				)
			}
			if (producer.language !== 'dbt') {
				return fail(`${script} writes this table but is not a dbt script, so there is no model to preview.`)
			}
			// A project whose descriptor interpolates `{{ }}` needs those values to
			// run at all, and this page has nowhere to ask for them. Say so instead
			// of submitting a job that fails inside dbt.
			const needs = (((producer.schema as any)?.required ?? []) as string[]).filter(
				(r) => r !== 'command'
			)
			if (needs.length > 0) {
				return fail(
					`${script} takes run arguments (${needs.join(', ')}), so previewing it needs the ` +
						`run form — open the script and run \`show\` there.`
				)
			}
			// dbt selects by MODEL name; the asset path ends in the relation's name,
			// which `alias`/`identifier` may have moved away from it. The graph
			// carries the provenance, so ask it rather than guessing from the path.
			//
			// Pinned to THIS script's version, which is what makes the answer the
			// right one: provenance keeps a single winner per relation workspace-wide,
			// so where two projects write one table the unpinned graph may name the
			// other project's model — and running this script with that selector
			// previews a model it does not have.
			const graph = await AssetService.getAssetsGraph({
				workspace: ws,
				assetKinds: 'dbt',
				dbtScriptHash: String(producer.hash)
			})
			const uniqueId = graph.assets?.find(
				(a: any) => a.kind === asset.kind && a.path === asset.path
			)?.dbt?.unique_id
			if (!uniqueId) {
				return fail(
					`${script} writes this table but its deployed version builds no model for it, ` +
						`so there is nothing to preview.`
				)
			}
			if (seq !== previewSeq) return
			const res = await previewDbtRows({
				workspace: ws,
				scriptPath: script,
				// The SAME version the selector came from: a deploy between the
				// provenance lookup and this submission would otherwise run a newer
				// project with the older one's model id, previewing a renamed
				// relation or failing outright for one since removed.
				scriptHash: producer.hash,
				model: nodeSelector(uniqueId),
				stillWanted: () => seq === previewSeq
			})
			if (seq === previewSeq && res) preview = res
		} catch (e) {
			fail(e instanceof Error ? e.message : String(e))
		}
	}


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
				d.map((d) => d.name).map((d) => ({ label: d == 'main' ? 'Main data table' : d, value: d }))
			)
	)
	let allVolumes = resource(
		() => $workspaceStore,
		() => VolumeService.listVolumes({ workspace: $workspaceStore! })
	)
	let volumesDrawer: VolumesDrawer | undefined = $state()

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
			<div class="flex flex-wrap gap-4">
				{#snippet card(props: {
					title: string
					assetKind: AssetKind
					data: ResourceReturn<{ label: string; value: string }[]>
					settingsHref: string
					docsHref: string
					favorites?: { table: string; schema?: string; assetName: string; path: string }[]
					itemExtra?: import('svelte').Snippet<[{ label: string; value: string }]>
				})}
					<div
						class="flex flex-col bg-surface-tertiary drop-shadow-base rounded-md grow basis-[340px] min-w-0"
					>
						<div class="flex flex-wrap justify-between items-center gap-2 border-b pt-5 px-6 pb-4">
							<h3 class="text-sm font-bold min-w-0 truncate" title={props.title}>{props.title}</h3>
							<div class="flex items-center h-fit gap-2 shrink-0">
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
								{#if !($userStore?.operator || (!$userStore?.is_admin && !$superadmin))}
									<Button
										wrapperClasses="h-fit"
										variant={props.data.current?.length === 0 && !props.data.loading
											? 'accent'
											: 'subtle'}
										iconOnly
										endIcon={{ icon: SettingsIcon }}
										href={props.settingsHref}
									/>
								{/if}
							</div>
						</div>
						{#if props.data.current?.length}
							<div class="max-h-96 overflow-y-auto pb-1">
								{#each props.data.current ?? [] as item}
									<div
										class="text-xs py-2 text-primary flex justify-between items-center gap-2 px-6"
									>
										<span class="min-w-0 truncate" title={item.label}>{item.label}</span>
										<div class="flex items-center gap-2 shrink-0">
											{#if props.itemExtra}
												{@render props.itemExtra(item)}
											{/if}
											<ExploreAssetButton
												asset={{ kind: props.assetKind, path: item.value }}
												{s3FilePicker}
												btnClasses="dark:bg-surface"
											/>
										</div>
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
					settingsHref: '/workspace_settings?tab=ducklake',
					docsHref: 'https://www.windmill.dev/docs/core_concepts/persistent_storage/ducklake',
					favorites: extractFavorites('ducklake')
				})}
				{#snippet volumesButton(item: { label: string; value: string })}
					{#if item.value === '/'}
						<Button
							variant="default"
							unifiedSize="sm"
							btnClasses="dark:bg-surface"
							startIcon={{ icon: HardDriveIcon }}
							on:click={() => volumesDrawer?.openDrawer()}
						>
							{allVolumes.current?.length ?? 0}
							{(allVolumes.current?.length ?? 0) === 1 ? 'volume' : 'volumes'}
						</Button>
					{/if}
				{/snippet}
				{@render card({
					title: 'Object storage',
					data: allS3Storages,
					assetKind: 's3object',
					settingsHref: '/workspace_settings?tab=windmill_lfs',
					docsHref:
						'https://www.windmill.dev/docs/core_concepts/persistent_storage/large_data_files',
					itemExtra: volumesButton
				})}
			</div>
		</Section>
		<Section label="Latest assets used" headerClass="whitespace-nowrap shrink-0">
			{#snippet action()}
				<div class="flex gap-2 grow justify-end min-w-0 ml-4">
					<RefreshButton
						variant="default"
						onClick={() => assetsQuery.reset()}
						loading={assetsQuery.isLoading}
					/>
					<FilterSearchbar
						class="grow max-w-[26rem] min-w-0"
						schema={assetsFilterSchema}
						bind:value={filterValues.val}
						placeholder="Filter assets..."
					/>
				</div>
			{/snippet}
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
<S3FilePicker bind:this={s3FilePicker} readOnlyMode allowDelete />
<VolumesDrawer
	bind:this={volumesDrawer}
	onExplore={async (name) => {
		const storage =
			(await VolumeService.getVolumeStorage({ workspace: $workspaceStore! })) ?? undefined
		s3FilePicker?.open({ s3: `volumes/${$workspaceStore}/${name}/`, storage })
	}}
/>

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
							{#snippet text()}
								{formatAssetKind(asset)}
							{/snippet}
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
							<ExploreAssetButton {asset} {s3FilePicker} _resourceMetadata={asset.metadata} />
						{/if}
						{#if dbtProducerOf(asset) && !$userStore?.operator}
							<Button
								size="xs2"
								color="light"
								variant="border"
								startIcon={{ icon: TableProperties }}
								on:click={() => previewTable(asset)}
							>
								Preview
							</Button>
						{/if}
						{#if asset.kind === 'resource' && asset.metadata === undefined}
							<Tooltip class={'w-24 flex items-center justify-center'}>
								<AlertTriangle size={20} class="text-orange-600 dark:text-orange-500" />
								{#snippet text()}
									Could not find resource
								{/snippet}
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

<Drawer bind:this={previewDrawer} size="800px">
	<DrawerContent title={`Rows of ${previewTitle}`} on:close={() => previewDrawer?.closeDrawer()}>
		{#if preview && 'pending' in preview}
			<div class="flex items-center gap-2 text-xs text-secondary">
				<Loader2 size={14} class="animate-spin" /> Previewing the first rows…
			</div>
		{:else if preview && 'error' in preview}
			<div class="text-xs text-red-600 dark:text-red-400">{preview.error}</div>
		{:else if preview && 'rows' in preview}
			{#if preview.rows.length === 0}
				<div class="text-xs text-secondary">No rows.</div>
			{:else}
				<div class="overflow-auto">
					<table class="text-xs w-full">
						<thead>
							<tr>
								{#each Object.keys(preview.rows[0]) as col}
									<th class="text-left px-2 py-1 font-semibold border-b">{col}</th>
								{/each}
							</tr>
						</thead>
						<tbody>
							{#each preview.rows as row}
								<tr>
									{#each Object.keys(preview.rows[0]) as col}
										<td class="px-2 py-1 border-b font-mono">{cell(row[col])}</td>
									{/each}
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			{/if}
		{/if}
	</DrawerContent>
</Drawer>
