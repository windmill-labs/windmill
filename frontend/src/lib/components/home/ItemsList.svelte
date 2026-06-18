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
	import { ChevronsDownUp, ChevronsUpDown } from 'lucide-svelte'

	import SearchItems from '../SearchItems.svelte'
	import NoItemFound from './NoItemFound.svelte'
	import { canWrite, getLocalSetting, storeLocalSetting } from '$lib/utils'
	import Drawer from '../common/drawer/Drawer.svelte'
	import HighlightCode from '../HighlightCode.svelte'
	import DrawerContent from '../common/drawer/DrawerContent.svelte'
	import Item from './Item.svelte'
	import TreeViewRoot from './TreeViewRoot.svelte'
	import { tick, untrack } from 'svelte'
	import { triggerableByAI } from '$lib/actions/triggerableByAI.svelte'
	import type { HomeFilterValue, LatestItem } from './homeFilter'

	interface Props {
		filterValue: HomeFilterValue
		itemKind?: 'all' | 'script' | 'flow' | 'app'
		showEditButtons?: boolean
		onMeta?: (meta: {
			owners: string[]
			labels: string[]
			loading: boolean
			latest: LatestItem[]
		}) => void
	}

	let { filterValue, itemKind = 'all', showEditButtons = true, onMeta }: Props = $props()

	// Filter inputs are owned by the parent toolbar (FilterSearchbar) and read
	// from `filterValue`. The list itself stays the source of truth for the
	// loaded items, the fuzzy search, and keyboard navigation.
	let filter = $derived(filterValue._default_ ?? '')
	let ownerFilter = $derived(filterValue.path)
	let labelFilters = $derived(
		(filterValue.label ?? '')
			.split(',')
			.map((l) => l.trim())
			.filter(Boolean)
	)
	let archived = $derived(!!filterValue.archived)
	let includeWithoutMain = $derived(!filterValue.exclude_library)
	let filterUserFolders = $derived(!!filterValue.user_folders_only)

	type TableItem<T, U extends 'script' | 'flow' | 'app' | 'raw_app'> = T & {
		canWrite: boolean
		marked?: string
		type?: U
		time?: number
		starred?: boolean
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

	const TREE_VIEW_SETTING_NAME = 'treeView'
	let treeView = $state(getLocalSetting(TREE_VIEW_SETTING_NAME) == 'true')
	let filterUserFoldersType: 'only f/*' | 'u/username and f/*' | undefined = $derived(
		$userStore?.is_super_admin && $userStore.username.includes('@')
			? 'only f/*'
			: $userStore?.is_admin || $userStore?.is_super_admin
				? 'u/username and f/*'
				: undefined
	)

	let viewCodeDrawer: Drawer | undefined = $state()
	let viewCodeTitle: string | undefined = $state()
	let script: Script | undefined = $state()
	async function showCode(path: string, summary: string) {
		viewCodeTitle = summary || path
		await viewCodeDrawer?.openDrawer()
		// `getDraft: true` so draft-only scripts (no deployed row at this
		// path) still return their content via the per-user draft overlay
		// instead of 404'ing.
		script = await ScriptService.getScriptByPath({
			workspace: $workspaceStore!,
			path,
			getDraft: true
		})
	}

	let collapseAll = $state(true)
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
	function itemLabels(x: { labels?: string[]; inherited_labels?: string[] }): string[] {
		return [...(x.labels ?? []), ...(x.inherited_labels ?? [])]
	}
	let owners = $derived(
		Array.from(
			new Set(combinedItems?.map((x) => x.path.split('/').slice(0, 2).join('/')) ?? [])
		).sort()
	)
	let allLabels = $derived(
		Array.from(new Set(combinedItems?.flatMap((x) => itemLabels(x)) ?? [])).sort()
	)
	// Latest-edited items for the home hero, by recency only (ignoring the
	// starred-first ordering used for the main list).
	let latestEdited = $derived(
		[...(combinedItems ?? [])]
			.sort((a, b) => (b.time ?? 0) - (a.time ?? 0))
			.slice(0, 6) as LatestItem[]
	)
	// Surface the owners/labels/loading/latest the parent toolbar + hero need to
	// build the FilterSearchbar schema, presets, and "Latest edited" cards.
	$effect(() => {
		onMeta?.({ owners, labels: allLabels, loading, latest: latestEdited })
	})

	let preFilteredItems = $derived(
		combinedItems?.filter(
			(x) =>
				(ownerFilter == undefined || x.path.startsWith(ownerFilter + '/')) &&
				(x.type == itemKind || itemKind == 'all') &&
				filterItemsPathsBaseOnUserFilters(x, filterUserFolders, filterUserFoldersType) &&
				(labelFilters.length == 0 || labelFilters.every((l) => itemLabels(x).includes(l)))
		)
	)
	let items = $derived(filter !== '' ? filteredItems : preFilteredItems)
	let displayedItems = $derived((items ?? []).slice(0, nbDisplayed))
	$effect(() => {
		items && resetScroll()
	})

	let selectedIndex: number = $state(-1)
	let hasMore = $derived(items != undefined && items.length > nbDisplayed)
	let loadMoreIndex = $derived(displayedItems.length)
	let loadMoreEl: HTMLButtonElement | undefined = $state()
	let pendingAutoSelect = $state(true)
	let firstWorkspaceRun = true
	$effect(() => {
		$workspaceStore
		pendingAutoSelect = true
		if (firstWorkspaceRun) {
			firstWorkspaceRun = false
			return
		}
	})
	$effect(() => {
		filter
		itemKind
		ownerFilter
		labelFilters
		archived
		// Skip while pendingAutoSelect is true (initial load / workspace switch);
		// the auto-select effect below will set the index once items appear.
		if (!pendingAutoSelect) {
			selectedIndex = -1
		}
	})
	$effect(() => {
		if (pendingAutoSelect && displayedItems.length > 0) {
			selectedIndex = 0
			pendingAutoSelect = false
		}
	})
	$effect(() => {
		const max = hasMore ? displayedItems.length : displayedItems.length - 1
		if (selectedIndex > max) {
			selectedIndex = max
		}
	})
	$effect(() => {
		if (hasMore && selectedIndex === loadMoreIndex) {
			loadMoreEl?.scrollIntoView({ block: 'nearest' })
		}
	})
	// Capture-phase listener so we run before melt-ui's button keydown handlers
	// (e.g. ArrowDown on the dropdown trigger would otherwise open the menu).
	$effect(() => {
		window.addEventListener('keydown', handleGlobalKeydown, true)
		return () => window.removeEventListener('keydown', handleGlobalKeydown, true)
	})

	function loadMoreAndPreselectFirstNew() {
		const previousNbDisplayed = nbDisplayed
		nbDisplayed += 30
		selectedIndex = previousNbDisplayed
	}

	function getSelectedRowActionButtons(): HTMLElement[] {
		const anchor = document.querySelector<HTMLElement>('a[data-row-keyboard-selected="true"]')
		const actions = anchor?.parentElement?.querySelector<HTMLElement>('[data-row-actions]')
		return actions ? Array.from(actions.querySelectorAll<HTMLElement>('button, a[href]')) : []
	}

	function handleGlobalKeydown(e: KeyboardEvent) {
		if (treeView) return
		const target = e.target as HTMLElement | null

		// When focus is inside a row's action buttons, handle arrow keys ourselves:
		//  - Left/Right cycle between buttons.
		//  - Up/Down move to the same-position button on the previous/next row.
		// All other keys pass through so Enter/Space activate the focused button normally.
		// This must run BEFORE the skipSelector check, since the dropdown ellipsis
		// trigger carries [data-menu] (which would otherwise filter the event out).
		// Up/Down also need stopImmediatePropagation so melt-ui's dropdown trigger
		// doesn't open the menu (its default ArrowDown behavior).
		const actionsContainer = target?.closest<HTMLElement>('[data-row-actions]')
		if (actionsContainer) {
			if (
				e.key !== 'ArrowRight' &&
				e.key !== 'ArrowLeft' &&
				e.key !== 'ArrowUp' &&
				e.key !== 'ArrowDown'
			)
				return
			const buttons = Array.from(actionsContainer.querySelectorAll<HTMLElement>('button, a[href]'))
			const currentIdx = buttons.indexOf(target as HTMLElement)
			if (currentIdx < 0) return
			if (e.key === 'ArrowRight') {
				if (currentIdx < buttons.length - 1) {
					e.preventDefault()
					buttons[currentIdx + 1].focus()
				}
			} else if (e.key === 'ArrowLeft') {
				e.preventDefault()
				if (currentIdx > 0) {
					buttons[currentIdx - 1].focus()
				}
			} else {
				// ArrowUp / ArrowDown: move to same-position button on prev/next row.
				e.preventDefault()
				e.stopImmediatePropagation()
				if (selectedIndex < 0 || selectedIndex >= displayedItems.length) return
				const newIndex =
					e.key === 'ArrowDown'
						? Math.min(selectedIndex + 1, displayedItems.length - 1)
						: Math.max(selectedIndex - 1, 0)
				if (newIndex === selectedIndex) return
				selectedIndex = newIndex
				tick().then(() => {
					const newButtons = getSelectedRowActionButtons()
					if (newButtons.length === 0) return
					const targetIdx = Math.min(currentIdx, newButtons.length - 1)
					newButtons[targetIdx]?.focus()
				})
			}
			return
		}

		// Inside an open dropdown menu: ArrowUp on first item / ArrowDown on last item
		// closes the menu (so users can leave with arrows instead of needing Escape).
		// Other arrow keys fall through to melt-ui's default cycle.
		const menuItem = target?.closest<HTMLElement>('[role="menuitem"]')
		if (menuItem) {
			if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
				const menu = menuItem.closest<HTMLElement>('[role="menu"]')
				if (menu) {
					const items = Array.from(menu.querySelectorAll<HTMLElement>('[role="menuitem"]'))
					const idx = items.indexOf(menuItem)
					const isFirst = idx === 0
					const isLast = idx === items.length - 1
					if ((e.key === 'ArrowUp' && isFirst) || (e.key === 'ArrowDown' && isLast)) {
						e.preventDefault()
						e.stopImmediatePropagation()
						menuItem.dispatchEvent(
							new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true })
						)
					}
				}
			}
			return
		}

		const skipSelector =
			'[role="menu"], [role="menuitem"], [role="dialog"], [role="listbox"], [role="combobox"], [aria-expanded="true"], [data-menu], [data-chat-keyboard-scope]'
		if (target) {
			const tag = target.tagName
			const isEditable =
				tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT' || target.isContentEditable
			if (isEditable) return
			if (target.closest(skipSelector)) return
		}
		const active = document.activeElement as HTMLElement | null
		if (active?.closest(skipSelector)) return

		// ArrowRight from body → focus first action button of selected row.
		if (e.key === 'ArrowRight') {
			if (selectedIndex < 0 || selectedIndex >= displayedItems.length) return
			const buttons = getSelectedRowActionButtons()
			if (buttons.length > 0) {
				e.preventDefault()
				buttons[0].focus()
			}
			return
		}

		if (e.key === 'ArrowDown') {
			if (displayedItems.length === 0) return
			e.preventDefault()
			if (selectedIndex === -1) {
				selectedIndex = 0
			} else if (selectedIndex === loadMoreIndex && hasMore) {
				selectedIndex = 0
			} else if (selectedIndex === displayedItems.length - 1) {
				selectedIndex = hasMore ? loadMoreIndex : 0
			} else {
				selectedIndex = selectedIndex + 1
			}
		} else if (e.key === 'ArrowUp') {
			if (displayedItems.length === 0) return
			e.preventDefault()
			if (selectedIndex === -1) {
				selectedIndex = displayedItems.length - 1
			} else if (selectedIndex === loadMoreIndex && hasMore) {
				selectedIndex = displayedItems.length - 1
			} else if (selectedIndex === 0) {
				selectedIndex = hasMore ? loadMoreIndex : displayedItems.length - 1
			} else {
				selectedIndex = selectedIndex - 1
			}
		} else if (e.key === 'Enter') {
			if (selectedIndex === loadMoreIndex && hasMore) {
				e.preventDefault()
				loadMoreAndPreselectFirstNew()
			} else if (selectedIndex >= 0 && selectedIndex < displayedItems.length) {
				const anchor = document.querySelector<HTMLAnchorElement>(
					'a[data-row-keyboard-selected="true"]'
				)
				if (anchor) {
					e.preventDefault()
					anchor.click()
				}
			}
		} else if (e.key === 'Escape') {
			if (selectedIndex !== -1) {
				e.preventDefault()
				selectedIndex = -1
			}
		}
	}
	$effect(() => {
		storeLocalSetting(TREE_VIEW_SETTING_NAME, treeView ? 'true' : undefined)
	})

	function reloadAll() {
		loadScripts(includeWithoutMain)
		loadFlows()
		loadApps()
		loadRawApps()
	}
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
		use:triggerableByAI={{
			id: 'home-items-list',
			description: 'Lists of scripts, flows, and apps'
		}}
	>
		{#if !loading}
			<div class="flex w-full flex-row-reverse gap-2 mb-1 items-center h-6">
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
				on:reload={reloadAll}
				{showCode}
			/>
		{:else}
			<div class="border rounded-md bg-surface-tertiary">
				{#each displayedItems as item, i (item.type + '/' + item.path + (item.hash ? '/' + item.hash : ''))}
					<Item
						{item}
						on:scriptChanged={() => loadScripts(includeWithoutMain)}
						on:flowChanged={loadFlows}
						on:appChanged={loadApps}
						on:rawAppChanged={loadRawApps}
						on:reload={reloadAll}
						{showCode}
						showEditButton={showEditButtons}
						keyboardSelected={selectedIndex === i}
					/>
				{/each}
			</div>
			{#if items && items?.length > 15 && nbDisplayed < items.length}
				<span class="text-xs font-normal text-secondary"
					>{nbDisplayed} items out of {items.length}
					<button
						bind:this={loadMoreEl}
						class="ml-4 text-xs font-normal text-primary hover:text-emphasis rounded px-1 {selectedIndex ===
						loadMoreIndex
							? 'bg-gray-200 dark:bg-gray-700 underline'
							: ''}"
						onclick={() => (nbDisplayed += 30)}>load 30 more</button
					></span
				>
			{/if}
		{/if}
	</div>
</CenteredPage>
