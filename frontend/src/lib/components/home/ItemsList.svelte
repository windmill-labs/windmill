<script lang="ts">
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { Button, Skeleton } from '$lib/components/common'
	import Toggle from '$lib/components/Toggle.svelte'
	import {
		AppService,
		FlowService,
		type ListableApp,
		type Script,
		ScriptService,
		type Flow,
		type ListableRawApp
	} from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import type uFuzzy from '@leeoniya/ufuzzy'
	import {
		ChevronsDownUp,
		ChevronsUpDown,
		Code2,
		LayoutDashboard,
		ListFilterPlus,
		SearchCode
	} from 'lucide-svelte'

	import { HOME_SEARCH_SHOW_FLOW, HOME_SEARCH_PLACEHOLDER } from '$lib/consts'

	import SearchItems from '../SearchItems.svelte'
	import ListFilters from './ListFilters.svelte'
	import NoItemFound from './NoItemFound.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import FlowIcon from './FlowIcon.svelte'
	import { canWrite, getLocalSetting, storeLocalSetting } from '$lib/utils'
	import { page } from '$app/stores'
	import { setQuery } from '$lib/navigation'
	import Drawer from '../common/drawer/Drawer.svelte'
	import HighlightCode from '../HighlightCode.svelte'
	import DrawerContent from '../common/drawer/DrawerContent.svelte'
	import Item from './Item.svelte'
	import TreeViewRoot from './TreeViewRoot.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { getContext, untrack } from 'svelte'
	import { triggerableByAI } from '$lib/actions/triggerableByAI.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	interface Props {
		filter?: string
		subtab?: 'flow' | 'script' | 'app'
		showEditButtons?: boolean
	}

	let { filter = $bindable(''), subtab = $bindable('script'), showEditButtons = true }: Props = $props()

	type TableItem<T, U extends 'script' | 'flow' | 'app' | 'raw_app'> = T & {
		canWrite: boolean
		marked?: string
		type?: U
		time?: number
		starred?: boolean
		has_draft?: boolean
		hash?: string
	}

	type TableScript = TableItem<Script, 'script'>
	type TableFlow = TableItem<Flow, 'flow'>
	type TableApp = TableItem<ListableApp, 'app'>
	type TableRawApp = TableItem<ListableRawApp, 'raw_app'>

	let scripts: TableScript[] | undefined = $state()
	let flows: TableFlow[] | undefined = $state()
	let apps: TableApp[] | undefined = $state()
	let raw_apps: TableRawApp[] | undefined = $state()

	let filteredItems: (TableScript | TableFlow | TableApp | TableRawApp)[] = $state([])

	let itemKind = $state(
		($page.url.searchParams.get('kind') as 'script' | 'flow' | 'app' | 'all') ?? 'all'
	)

	let loading = $state(true)

	let nbDisplayed = $state(15)

	async function loadScripts(includeWithoutMain: boolean): Promise<void> {
		const loadedScripts = await ScriptService.listScripts({
			workspace: $workspaceStore!,
			showArchived: archived ? true : undefined,
			includeWithoutMain: includeWithoutMain ? true : undefined,
			includeDraftOnly: true,
			withoutDescription: true
		})

		scripts = loadedScripts.map((script: Script) => {
			return {
				canWrite: canWrite(script.path, script.extra_perms, $userStore) && !$userStore?.operator,
				...script
			}
		})
		loading = false
	}

	async function loadFlows(): Promise<void> {
		flows = (
			await FlowService.listFlows({
				workspace: $workspaceStore!,
				showArchived: archived ? true : undefined,
				includeDraftOnly: true,
				withoutDescription: true
			})
		).map((x: Flow) => {
			return {
				canWrite:
					canWrite(x.path, x.extra_perms, $userStore) &&
					x.workspace_id == $workspaceStore &&
					!$userStore?.operator,
				...x
			}
		})
		loading = false
	}

	async function loadApps(): Promise<void> {
		apps = (await AppService.listApps({ workspace: $workspaceStore!, includeDraftOnly: true })).map(
			(app: ListableApp) => {
				return {
					canWrite:
						canWrite(app.path!, app.extra_perms!, $userStore) &&
						app.workspace_id == $workspaceStore &&
						!$userStore?.operator,
					...app
				}
			}
		)
		loading = false
	}

	async function loadRawApps(): Promise<void> {
		raw_apps = []
		loading = false
	}

	function filterItemsPathsBaseOnUserFilters(
		item: TableScript | TableFlow | TableApp | TableRawApp,
		filterUserFolders: boolean,
		filterUserFoldersType: 'only f/*' | 'u/username and f/*' | undefined
	) {
		if (!filterUserFoldersType || !filterUserFolders) return true
		if (filterUserFoldersType === 'only f/*') return item.path.startsWith('f/')
		if (filterUserFoldersType === 'u/username and f/*')
			return item.path.startsWith('f/') || item.path.startsWith(`u/${$userStore?.username}/`)
		return true // should not happen
	}

	let ownerFilter: string | undefined = $state(undefined)

	const cmp = new Intl.Collator('en').compare

	const opts: uFuzzy.Options = {
		sort: (info, haystack, needle) => {
			let {
				idx,
				chars,
				terms,
				interLft2,
				interLft1,
				//	interRgt2,
				//	interRgt1,
				start,
				intraIns,
				interIns
			} = info

			const sortResult = idx
				.map((v, i) => i)
				.sort(
					(ia, ib) =>
						// most contig chars matched
						chars[ib] - chars[ia] ||
						// least char intra-fuzz (most contiguous)
						intraIns[ia] - intraIns[ib] ||
						// most prefix bounds, boosted by full term matches
						terms[ib] +
							interLft2[ib] +
							0.5 * interLft1[ib] -
							(terms[ia] + interLft2[ia] + 0.5 * interLft1[ia]) ||
						// highest density of match (least span)
						//	span[ia] - span[ib] ||
						// highest density of match (least term inter-fuzz)
						interIns[ia] - interIns[ib] ||
						// earliest start of match
						start[ia] - start[ib] ||
						// alphabetic
						cmp(haystack[idx[ia]], haystack[idx[ib]]) +
							(preFilteredItems?.[idx[ib]]?.starred ? 100 : 0) -
							(preFilteredItems?.[idx[ia]]?.starred ? 100 : 0)
				)
			return sortResult
		}
	}

	function resetScroll() {
		const element = document.getElementsByTagName('svelte-virtual-list-viewport')
		const firstElement = element.item(0)
		if (firstElement) {
			firstElement.scrollTop = 0
		}
	}

	let archived = $state(false)

	const TREE_VIEW_SETTING_NAME = 'treeView'
	const FILTER_USER_FOLDER_SETTING_NAME = 'filterUserFolders'
	const INCLUDE_WITHOUT_MAIN_SETTING_NAME = 'includeWithoutMain'
	let treeView = $state(getLocalSetting(TREE_VIEW_SETTING_NAME) == 'true')
	let filterUserFoldersType: 'only f/*' | 'u/username and f/*' | undefined = $derived(
		$userStore?.is_super_admin && $userStore.username.includes('@')
			? 'only f/*'
			: $userStore?.is_admin || $userStore?.is_super_admin
				? 'u/username and f/*'
				: undefined
	)
	let filterUserFolders = $state(getLocalSetting(FILTER_USER_FOLDER_SETTING_NAME) == 'true')
	let includeWithoutMain = $state(
		getLocalSetting(INCLUDE_WITHOUT_MAIN_SETTING_NAME)
			? getLocalSetting(INCLUDE_WITHOUT_MAIN_SETTING_NAME) == 'true'
			: true
	)

	const openSearchWithPrefilledText: (t?: string) => void = getContext(
		'openSearchWithPrefilledText'
	)

	let viewCodeDrawer: Drawer | undefined = $state()
	let viewCodeTitle: string | undefined = $state()
	let script: Script | undefined = $state()
	async function showCode(path: string, summary: string) {
		viewCodeTitle = summary || path
		await viewCodeDrawer?.openDrawer()
		script = await ScriptService.getScriptByPath({
			workspace: $workspaceStore!,
			path
		})
	}

	let collapseAll = $state(true)
	let owners = $derived(
		Array.from(
			new Set(filteredItems?.map((x) => x.path.split('/').slice(0, 2).join('/')) ?? [])
		).sort()
	)
	$effect(() => {
		if ($userStore && $workspaceStore) {
			;[archived, includeWithoutMain]
			untrack(() => {
				loadScripts(includeWithoutMain)
				loadFlows()
				if (!archived) {
					loadApps()
					loadRawApps()
				} else {
					apps = []
					raw_apps = []
				}
			})
		}
	})

	let combinedItems = $derived(
		flows == undefined || scripts == undefined || apps == undefined || raw_apps == undefined
			? undefined
			: [
					...flows.map((x) => ({
						...x,
						type: 'flow' as 'flow',
						time: new Date(x.edited_at).getTime()
					})),
					...scripts.map((x) => ({
						...x,
						type: 'script' as 'script',
						time: new Date(x.created_at).getTime()
					})),
					...apps.map((x) => ({
						...x,
						type: 'app' as 'app',
						time: new Date(x.edited_at).getTime()
					})),
					...raw_apps.map((x) => ({
						...x,
						type: 'raw_app' as 'raw_app',
						time: new Date(x.edited_at).getTime()
					}))
				].sort((a, b) =>
					a.starred != b.starred ? (a.starred ? -1 : 1) : a.time - b.time > 0 ? -1 : 1
				)
	)
	$effect(() => {
		if ($workspaceStore) {
			ownerFilter = undefined
		}
	})
	let preFilteredItems = $derived(
		ownerFilter != undefined
			? combinedItems?.filter(
					(x) =>
						x.path.startsWith(ownerFilter + '/') &&
						(x.type == itemKind || itemKind == 'all') &&
						filterItemsPathsBaseOnUserFilters(x, filterUserFolders, filterUserFoldersType)
				)
			: combinedItems?.filter(
					(x) =>
						(x.type == itemKind || itemKind == 'all') &&
						filterItemsPathsBaseOnUserFilters(x, filterUserFolders, filterUserFoldersType)
				)
	)
	let items = $derived(filter !== '' ? filteredItems : preFilteredItems)
	$effect(() => {
		items && resetScroll()
	})
	$effect(() => {
		storeLocalSetting(TREE_VIEW_SETTING_NAME, treeView ? 'true' : undefined)
	})
	$effect(() => {
		storeLocalSetting(FILTER_USER_FOLDER_SETTING_NAME, filterUserFolders ? 'true' : undefined)
	})
	$effect(() => {
		storeLocalSetting(INCLUDE_WITHOUT_MAIN_SETTING_NAME, includeWithoutMain ? 'true' : undefined)
	})
</script>

<SearchItems
	{filter}
	items={preFilteredItems}
	bind:filteredItems
	f={(x) => (x.summary ? x.summary + ' (' + x.path + ')' : x.path)}
	{opts}
/>

<Drawer
	bind:this={viewCodeDrawer}
	on:close={() => {
		setTimeout(() => {
			viewCodeTitle = undefined
			script = undefined
		}, 300)
	}}
>
	<DrawerContent title={viewCodeTitle} on:close={viewCodeDrawer.closeDrawer}>
		{#if script}
			<HighlightCode language={script?.language} code={script?.content} />
		{:else}
			<Skeleton layout={[[40]]} />
		{/if}
	</DrawerContent>
</Drawer>

<CenteredPage wrapperClasses="w-full" handleOverflow={false}>
	<div
		class="flex flex-wrap gap-2 items-center justify-between w-full"
		use:triggerableByAI={{
			id: 'home-items-list',
			description: 'Lists of scripts, flows, and apps'
		}}
	>
		<div class="flex justify-start">
			<ToggleButtonGroup
				bind:selected={itemKind}
				onSelected={(v) => {
					if (itemKind != 'all') {
						subtab = v
					}
					setQuery($page.url, 'kind', v)
				}}
			>
				{#snippet children({ item })}
					<ToggleButton value="all" label="All" size="md" {item} />
					<ToggleButton value="script" icon={Code2} label="Scripts" size="md" {item} />
					{#if HOME_SEARCH_SHOW_FLOW}
						<ToggleButton
							value="flow"
							label="Flows"
							icon={FlowIcon}
							selectedColor="#14b8a6"
							size="md"
							{item}
						/>
					{/if}
					<ToggleButton
						value="app"
						label="Apps"
						icon={LayoutDashboard}
						selectedColor="#fb923c"
						size="md"
						{item}
					/>
				{/snippet}
			</ToggleButtonGroup>
		</div>

		<div class="relative text-primary grow min-w-[100px]">
			<!-- svelte-ignore a11y_autofocus -->
			<TextInput
				inputProps={{
					autofocus: true,
					placeholder: HOME_SEARCH_PLACEHOLDER,
					id: 'home-search-input'
				}}
				size="md"
				bind:value={filter}
				class="!pr-10"
			/>
			<button aria-label="Search" type="submit" class="absolute right-0 top-0 mt-2 mr-4">
				<svg
					class="h-4 w-4 fill-current"
					xmlns="http://www.w3.org/2000/svg"
					xmlns:xlink="http://www.w3.org/1999/xlink"
					version="1.1"
					id="Capa_1"
					x="0px"
					y="0px"
					viewBox="0 0 56.966 56.966"
					style="enable-background:new 0 0 56.966 56.966;"
					xml:space="preserve"
					width="512px"
					height="512px"
				>
					<path
						d="M55.146,51.887L41.588,37.786c3.486-4.144,5.396-9.358,5.396-14.786c0-12.682-10.318-23-23-23s-23,10.318-23,23  s10.318,23,23,23c4.761,0,9.298-1.436,13.177-4.162l13.661,14.208c0.571,0.593,1.339,0.92,2.162,0.92  c0.779,0,1.518-0.297,2.079-0.837C56.255,54.982,56.293,53.08,55.146,51.887z M23.984,6c9.374,0,17,7.626,17,17s-7.626,17-17,17  s-17-7.626-17-17S14.61,6,23.984,6z"
					/>
				</svg>
			</button>
		</div>
		<Button
			on:click={() => openSearchWithPrefilledText('#')}
			variant="default"
			unifiedSize="md"
			endIcon={{
				icon: SearchCode
			}}
		>
			Content
		</Button>
	</div>
	<div class="relative">
		<ListFilters
			syncQuery
			bind:selectedFilter={ownerFilter}
			filters={owners}
			bottomMargin={false}
		/>
		{#if filteredItems?.length == 0}
			<div class="mt-10"></div>
		{/if}
		{#if !loading}
			<div class="flex w-full flex-row-reverse gap-2 mt-2 mb-1 items-center h-6">
				<Popover floatingConfig={{ placement: 'bottom-end' }}>
					{#snippet trigger()}
						<Button
							startIcon={{
								icon: ListFilterPlus
							}}
							nonCaptureEvent
							iconOnly
							size="xs"
							color="light"
							variant="default"
							spacingSize="xs2"
						/>
					{/snippet}
					{#snippet content()}
						<div class="p-4">
							<span class="text-sm font-semibold text-emphasis">Filters</span>
							<div class="flex flex-col gap-2 mt-2">
								<Toggle size="xs" bind:checked={archived} options={{ right: 'Only archived' }} />
								{#if $userStore && !$userStore.operator}
									<Toggle
										size="xs"
										bind:checked={includeWithoutMain}
										options={{ right: 'Include without main function' }}
									/>
								{/if}
							</div>
						</div>
					{/snippet}
				</Popover>
				{#if filterUserFoldersType === 'only f/*'}
					<Toggle size="xs" bind:checked={filterUserFolders} options={{ right: 'Only f/*' }} />
				{:else if filterUserFoldersType === 'u/username and f/*'}
					<Toggle
						size="xs"
						bind:checked={filterUserFolders}
						options={{ right: `Only u/${$userStore?.username} and f/*` }}
					/>
				{/if}
				<Toggle size="xs" bind:checked={treeView} options={{ right: 'Tree view' }} />
				{#if treeView}
					<Button
						unifiedSize="sm"
						variant="subtle"
						on:click={() => (collapseAll = !collapseAll)}
						startIcon={{
							icon: collapseAll ? ChevronsUpDown : ChevronsDownUp
						}}
					>
						{#if collapseAll}
							Expand all
						{:else}
							Collapse all
						{/if}
					</Button>
				{/if}
			</div>
		{/if}
	</div>
	<div>
		{#if filteredItems == undefined}
			<div class="mt-4"></div>
			<Skeleton layout={[[2], 1]} />
			{#each new Array(6) as _}
				<Skeleton layout={[[4], 0.5]} />
			{/each}
		{:else if filteredItems.length === 0}
			<NoItemFound hasFilters={filter !== '' || archived || filterUserFolders} />
		{:else if treeView}
			<TreeViewRoot
				{items}
				{nbDisplayed}
				{collapseAll}
				isSearching={filter !== ''}
				on:scriptChanged={() => loadScripts(includeWithoutMain)}
				on:flowChanged={loadFlows}
				on:appChanged={loadApps}
				on:rawAppChanged={loadRawApps}
				on:reload={() => {
					loadScripts(includeWithoutMain)
					loadFlows()
					loadApps()
					loadRawApps()
				}}
				{showCode}
			/>
		{:else}
			<div class="border rounded-md bg-surface-tertiary">
				{#each (items ?? []).slice(0, nbDisplayed) as item (item.type + '/' + item.path + (item.hash ? '/' + item.hash : ''))}
					<Item
						{item}
						on:scriptChanged={() => loadScripts(includeWithoutMain)}
						on:flowChanged={loadFlows}
						on:appChanged={loadApps}
						on:rawAppChanged={loadRawApps}
						on:reload={() => {
							loadScripts(includeWithoutMain)
							loadFlows()
							loadApps()
							loadRawApps()
						}}
						{showCode}
						showEditButton={showEditButtons}
					/>
				{/each}
			</div>
			{#if items && items?.length > 15 && nbDisplayed < items.length}
				<span class="text-xs font-normal text-secondary"
					>{nbDisplayed} items out of {items.length}
					<button
						class="ml-4 text-xs font-normal text-primary hover:text-emphasis"
						onclick={() => (nbDisplayed += 30)}>load 30 more</button
					></span
				>
			{/if}
		{/if}
	</div>
</CenteredPage>
