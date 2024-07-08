<script lang="ts">
	import { onDestroy, onMount, tick } from 'svelte'
	import {
		AppService,
		FlowService,
		IndexSearchService,
		RawAppService,
		ScriptService,
		type Flow,
		type ListableApp,
		type ListableRawApp,
		type Script
	} from '$lib/gen'
	import { clickOutside, displayDateOnly } from '$lib/utils'
	import TimeAgo from './TimeAgo.svelte'
	import {
		BoxesIcon,
		CalendarIcon,
		ChevronDown,
		Code2Icon,
		DollarSignIcon,
		HomeIcon,
		LayoutDashboardIcon,
		Loader2,
		PlayIcon,
		Search
	} from 'lucide-svelte'
	import JobPreview from './runs/JobPreview.svelte'
	import Portal from 'svelte-portal'
	import { twMerge } from 'tailwind-merge'
	import ContentSearchInner from './ContentSearchInner.svelte'
	import { goto } from '$app/navigation'
	import QuickMenuItem from './search/QuickMenuItem.svelte'
	import { enterpriseLicense, workspaceStore } from '$lib/stores'
	import uFuzzy from '@leeoniya/ufuzzy'
	import DropdownV2 from './DropdownV2.svelte'
	import BarsStaggered from './icons/BarsStaggered.svelte'
	import { scroll_into_view_if_needed_polyfill } from './multiselect/utils'
	import { Alert } from './common'

	export let open: boolean = false

	let searchTerm: string = ''
	let textInput: HTMLInputElement
	let selectedWorkspace: string | undefined = undefined
	let contentSearch: ContentSearchInner | undefined = undefined

	const RUNS_PREFIX = '>'
	const LOGS_PREFIX = '!'
	const CONTENT_SEARCH_PREFIX = '#'
	const SWITCH_MODE_PREFIX = '?'

	type SearchMode = 'default' | 'switch-mode' | 'runs' | 'content' | 'logs'

	let tab: SearchMode = 'default'

	type quickMenuItem = {
		search_id: string
		label: string
		action: () => void
		icon?: any
		charIcon?: string
	}
	let switchModeItems: quickMenuItem[] = [
		{
			search_id: 'switchto:run-search',
			label: 'Search across completed runs',
			action: () => switchMode('runs'),
			charIcon: RUNS_PREFIX
		},
		{
			search_id: 'switchto:content-search',
			label: 'Search scripts/flows/apps based on content',
			action: () => switchMode('content'),
			charIcon: CONTENT_SEARCH_PREFIX
		},
		{
			search_id: 'switchto:log-search',
			label: 'Search windmill logs',
			action: () => switchMode('logs'),
			charIcon: LOGS_PREFIX
		}
	]
	let defaultMenuItems: quickMenuItem[] = [
		{ search_id: 'nav:home', label: 'Go to Home', action: () => gotoPage('/'), icon: HomeIcon },
		{ search_id: 'nav:runs', label: 'Go to Runs', action: () => gotoPage('/runs'), icon: PlayIcon },
		{
			search_id: 'nav:variables',
			label: 'Go to Variables',
			action: () => gotoPage('/variables'),
			icon: DollarSignIcon
		},
		{
			search_id: 'nav:resources',
			label: 'Go to Resources',
			action: () => gotoPage('/resources'),
			icon: BoxesIcon
		},
		{
			search_id: 'nav:schedules',
			label: 'Go to Schedules',
			action: () => gotoPage('/schedules'),
			icon: CalendarIcon
		},
		...switchModeItems
	]

	let itemMap = {
		default: defaultMenuItems as any[],
		'switch-mode': switchModeItems,
		runs: [] as any[],
		content: [] as any[],
		logs: [] as any[]
	}

	$: tab === 'content' && contentSearch?.open()

	async function switchPrompt(tab: string) {
		if (tab === 'default') {
			searchTerm = ''
		}
		if (tab === 'runs') {
			searchTerm = RUNS_PREFIX
		}
		if (tab === 'content') {
			searchTerm = CONTENT_SEARCH_PREFIX
		}
		if (tab === 'switch-mode') {
			searchTerm = SWITCH_MODE_PREFIX
		}
		if (tab === 'logs') {
			searchTerm = LOGS_PREFIX
		}
		selectedItem = selectItem(0)
		textInput.focus()
	}

	function removePrefix(str: string, prefix: string): string {
		if (str.startsWith(prefix)) {
			return str.substring(prefix.length)
		}
		return str
	}

	let opts: uFuzzy.Options = {}

	let uf = new uFuzzy(opts)
	let defaultMenuItemLabels = defaultMenuItems.map((item) => item.label)
	let switchModeItemLabels = switchModeItems.map((item) => item.label)

	function fuzzyFilter(filter: string, items: any[], itemsPlainText: string[]) {
		if (filter === '') {
			return items
		}
		let idxs = uf.filter(itemsPlainText, filter) ?? []

		let info: uFuzzy.Info
		// parts is undefined error happens when filter is similar
		// to `.>!` (string with no letters but some symbols)
		try {
			info = uf.info(idxs, itemsPlainText, filter)
		} catch (e) {
			return items
		}
		let order = uf.sort(info, itemsPlainText, filter)

		let r: any[] = []
		for (let o of order) {
			r.push(items[info.idx[o]])
		}
		return r
	}

	let debounceTimeout: any = undefined
	const debouncePeriod: number = 1000
	let loadingCompletedRuns: boolean = false
	async function handleSearch() {
		if (
			tab !== 'default' &&
			(searchTerm === '' ||
				![RUNS_PREFIX, LOGS_PREFIX, CONTENT_SEARCH_PREFIX, SWITCH_MODE_PREFIX].includes(
					searchTerm[0]
				))
		) {
			_switchMode('default')
		}
		if (tab != 'switch-mode' && searchTerm.length > 0 && searchTerm[0] === SWITCH_MODE_PREFIX) {
			_switchMode('switch-mode')
		}
		if (tab != 'logs' && searchTerm.length > 0 && searchTerm[0] === LOGS_PREFIX) {
			_switchMode('logs')
		}
		if (tab != 'runs' && searchTerm.length > 0 && searchTerm[0] === RUNS_PREFIX) {
			_switchMode('runs')
		}
		if (tab != 'content' && searchTerm.length > 0 && searchTerm[0] === CONTENT_SEARCH_PREFIX) {
			_switchMode('content')
		}

		if (tab === 'default') {
			itemMap['default'] = fuzzyFilter(searchTerm, defaultMenuItems, defaultMenuItemLabels)
			if (combinedItems) {
				itemMap['default'] = itemMap['default'].concat(
					fuzzyFilter(
						searchTerm,
						combinedItems,
						combinedItems.map((i) => `${i.path} ${i.summary}`)
					)
				)
			}
		}
		if (tab === 'switch-mode') {
			itemMap['switch-mode'] = fuzzyFilter(
				removePrefix(searchTerm, SWITCH_MODE_PREFIX),
				switchModeItems,
				switchModeItemLabels
			)
		}
		if (tab === 'runs') {
			// only search if hasn't been called in some small time. Show load animation while wating. Also add a cache that resets everytime the modal is closed
			const s = removePrefix(searchTerm, RUNS_PREFIX)
			clearTimeout(debounceTimeout)
			loadingCompletedRuns = true
			debounceTimeout = setTimeout(async () => {
				clearTimeout(debounceTimeout)
				const searchResults = await IndexSearchService.searchJobsIndex({
					query: s,
					workspace: $workspaceStore!
				})
				loadingCompletedRuns = false
				itemMap['runs'] = searchResults.document_hits
				selectedItem = selectItem(0)
			}, debouncePeriod)
		}
		selectedItem = selectItem(0)
	}

	function selectItem(index: number) {
		if (!itemMap[tab] || itemMap[tab].length <= index) {
			return undefined
		}
		onHover(itemMap[tab][index])
		return itemMap[tab][index]
	}

	let selectedItem: any
	async function handleKeydown(event: KeyboardEvent) {
		if (event.ctrlKey && event.key === 'k') {
			event.preventDefault()
			open = !open
			await tick()
			if (open) {
				if (combinedItems == undefined) {
					combinedItems = await fetchCombinedItems()
					handleSearch()
				}
				selectedItem = selectItem(0)
				textInput.focus()
				textInput.select()
			}
		}
		if (open) {
			if (event.key === 'Escape') {
				event.preventDefault()
				if (searchTerm.length != 0 || tab != 'default') {
					switchMode('default')
					textInput?.focus()
				} else {
					open = false
				}
			}
			if (event.key === 'ArrowDown') {
				event.preventDefault()
				let idx = itemMap[tab].indexOf(selectedItem)
				if (idx != -1) {
					idx = (idx + 1) % itemMap[tab].length
					selectedItem = selectItem(idx)
					let el = document.getElementById(selectedItem.search_id)
					if (el) scroll_into_view_if_needed_polyfill(el, false)
				}
			} else if (event.key === 'ArrowUp') {
				event.preventDefault()
				let idx = itemMap[tab].indexOf(selectedItem)
				if (idx != -1) {
					idx = (idx - 1 + itemMap[tab].length) % itemMap[tab].length
					selectedItem = selectItem(idx)
					let el = document.getElementById(selectedItem.search_id)
					if (el) scroll_into_view_if_needed_polyfill(el, false)
				}
			}
		}
	}

	//internal, should not be called outside of the handleSearch function
	function _switchMode(mode: SearchMode) {
		selectedItem = undefined
		tab = mode
	}
	// Used by callbacks, call this to change the mode
	function switchMode(mode: SearchMode) {
		switchPrompt(mode)
		textInput.focus()
	}

	function gotoWindmillItemPage(e: TableAny) {
		let path: string
		switch (e.type) {
			case 'flow':
				path = `/flows/get/${e.path}`
				break
			case 'script':
				path = `/scripts/get/${e.path}`
				break
			case 'app':
				path = `/apps/get/${e.path}`
				break
			case 'raw_app':
				path = `/raw_apps/get/${e.path}`
				break
			default:
				path = '/'
		}
		gotoPage(path)
	}

	function gotoPage(path: string) {
		open = false
		searchTerm = ''
		goto(path)
	}

	onMount(() => {
		window.addEventListener('keydown', handleKeydown)
	})

	onDestroy(() => {
		window.removeEventListener('keydown', handleKeydown)
	})
	$: searchTerm, handleSearch()

	function placeholderForMode(mode: SearchMode): string | null | undefined {
		switch (mode) {
		}
		switch (mode) {
			case 'default':
				return 'Search or type `?` for search options'
			case 'runs':
				return 'Search across all jobs'
			default:
				return 'Search...'
		}
	}

	function searchModeDescription(mode: SearchMode) {
		switch (mode) {
			case 'runs':
				return 'completed runs'
			case 'logs':
				return 'service logs'
			case 'content':
				return 'flows/apps/scripts by content'
			case 'default':
				return 'flows/apps/scripts by name'
		}
		return ''
	}

	type TableItem<T, U extends 'script' | 'flow' | 'app' | 'raw_app'> = T & {
		search_id: string
		marked?: string
		type?: U
		time?: number
		starred?: boolean
		has_draft?: boolean
	}

	// interface SelectableSearchMenuItem {
	// 	search_id: string
	// }

	type TableScript = TableItem<Script, 'script'>
	type TableFlow = TableItem<Flow, 'flow'>
	type TableApp = TableItem<ListableApp, 'app'>
	type TableRawApp = TableItem<ListableRawApp, 'raw_app'>

	type TableAny = TableScript | TableFlow | TableApp | TableRawApp

	let combinedItems: TableAny[] | undefined = undefined

	async function fetchCombinedItems() {
		const scripts = await ScriptService.listScripts({
			workspace: $workspaceStore!
		})
		const flows = await FlowService.listFlows({
			workspace: $workspaceStore!
		})
		const apps = await AppService.listApps({ workspace: $workspaceStore! })
		const raw_apps = await RawAppService.listRawApps({ workspace: $workspaceStore! })

		let combinedItems: (TableScript | TableFlow | TableApp | TableRawApp)[] | undefined = [
			...flows.map((x) => ({
				...x,
				type: 'flow' as 'flow',
				time: new Date(x.edited_at).getTime(),
				search_id: x.path
			})),
			...scripts.map((x) => ({
				...x,
				type: 'script' as 'script',
				time: new Date(x.created_at).getTime(),
				search_id: x.path
			})),
			...apps.map((x) => ({
				...x,
				type: 'app' as 'app',
				time: new Date(x.edited_at).getTime(),
				search_id: x.path
			})),
			...raw_apps.map((x) => ({
				...x,
				type: 'raw_app' as 'raw_app',
				time: new Date(x.edited_at).getTime(),
				search_id: x.path
			}))
		].sort((a, b) => (a.starred != b.starred ? (a.starred ? -1 : 1) : a.time - b.time > 0 ? -1 : 1))

		return combinedItems
	}

	function iconForWindmillItem(type: string) {
		switch (type) {
			case 'flow':
				return BarsStaggered
			case 'script':
				return Code2Icon
			case 'app':
				return LayoutDashboardIcon
			case 'raw_app':
				return LayoutDashboardIcon
		}
	}

	function onHover(selectedItem: any) {
		if (tab === 'runs') {
			selectedWorkspace = selectedItem?.document?.workspace_id[0]
		}
	}
</script>

{#if open}
	<Portal>
		<div
			class={twMerge(
				`fixed top-0 bottom-0 left-0 right-0 transition-all duration-50`,
				' bg-black bg-opacity-60',
				'z-[1100]'
			)}
		>
			<div
				class={'max-w-4xl lg:mx-auto mx-10 mt-40 bg-surface rounded-lg relative'}
				use:clickOutside={false}
				on:click_outside={() => {
					open = false
				}}
			>
				<div class="px-4 py-2 items-center">
					<div class="quick-search-input flex flex-row gap-1 items-center mb-2">
						<Search />
						<input
							bind:this={textInput}
							type="text"
							class="quick-search-input"
							bind:value={searchTerm}
							placeholder={placeholderForMode(tab)}
						/>
					</div>
					<div class="overflow-scroll max-h-[30rem]">
						{#if tab === 'default' || tab === 'switch-mode'}
							{#each (itemMap[tab] ?? []).filter((e) => defaultMenuItems.includes(e)) as el}
								<QuickMenuItem
									on:select={el?.action}
									on:hover={() => (selectedItem = el)}
									id={el?.search_id}
									hovered={el?.search_id === selectedItem?.search_id}
									label={el?.label}
									icon={el?.icon}
									charIcon={el?.charIcon}
								/>
							{/each}
						{/if}
						{#if tab !== 'switch-mode'}
							<div
								class="flex flex-row items-center text-xs font-bold font-small-caps border-b-2 border-gray-300 p-1 mt-2"
							>
								<span class="h-6 px-1"> Search </span>
								<span>
									<DropdownV2
										items={[
											{
												displayName: searchModeDescription('default'),
												action: (e) => {
													e.stopPropagation()
													switchMode('default')
												}
											},
											{
												displayName: searchModeDescription('content'),
												action: (e) => {
													e.stopPropagation()
													switchMode('content')
												}
											},
											{
												displayName: searchModeDescription('runs'),
												action: (e) => {
													e.stopPropagation()
													switchMode('runs')
												}
											},
											{
												displayName: searchModeDescription('logs'),
												action: (e) => {
													e.stopPropagation()
													switchMode('logs')
												}
											}
										]}
									>
										<svelte:fragment slot="buttonReplacement">
											<div class="flex flex-row text-xs items-center">
												<div
													class="pl-1 h-6 flex flex-row items-center hover:bg-surface-hover cursor-pointer rounded-md"
												>
													<span class="">{searchModeDescription(tab)}</span>
													<ChevronDown class="w-5 h-5" />
												</div>
											</div>
										</svelte:fragment>
									</DropdownV2>
								</span>
							</div>
						{/if}
						{#if tab === 'default'}
							{#each (itemMap[tab] ?? []).filter((e) => (combinedItems ?? []).includes(e)) as el}
								<QuickMenuItem
									on:select={() => gotoWindmillItemPage(el)}
									on:hover={() => (selectedItem = el)}
									id={el?.search_id}
									hovered={el?.path === selectedItem?.path}
									label={(el.summary ? `${el.summary} - ` : '') +
										el.path +
										(el.starred ? ' ★' : '')}
									icon={iconForWindmillItem(el.type)}
								/>
							{/each}
							{#if (itemMap[tab] ?? []).length === 0}
								<div class="flex w-full justify-center items-center h-48">
									<div class="text-tertiary text-center">
										<div class="text-2xl font-bold">Nothing found</div>
										<div class="text-sm">Tip: press `esc` to quickly clear the search bar</div>
									</div>
								</div>
							{/if}
						{:else if tab === 'content'}
							<ContentSearchInner
								classNameInner="max-h-[20rem]"
								search={removePrefix(searchTerm, '#')}
								bind:this={contentSearch}
							/>
						{:else if tab === 'logs'}
							<div class="p-2">
								<Alert title="Service log search is coming soon" type="info">
									Full text search on windmill's service logs is coming soon
								</Alert>
							</div>
						{:else if tab === 'runs'}
							<div class="flex h-96">
								{#if loadingCompletedRuns}
									<div class="flex w-full justify-center items-center h-48">
										<div class="text-tertiary text-center">
											<Loader2 size={34} class="animate-spin" />
										</div>
									</div>
								{:else if itemMap['runs'] && itemMap['runs'].length > 0}
									<div class="w-5/12 overflow-scroll">
										{#each itemMap['runs'] ?? [] as r}
											<QuickMenuItem
												on:hover={() => {
													selectedItem = r
													selectedWorkspace = r?.document.workspace_id[0]
												}}
												on:select={() => {
													open = false
													goto(`/run/${r?.document.id[0]}`)
												}}
												id={r?.document.id[0]}
												hovered={selectedItem && r?.document.id[0] === selectedItem?.document.id[0]}
												icon={r?.icon}
											>
												<svelte:fragment slot="itemReplacement">
													<button
														class={twMerge(
															`w-full flex items-center justify-between gap-1 py-2 px-2 text-left border rounded-sm transition-a`,
															r?.document.id === selectedItem?.document?.id
																? 'bg-surface-hover'
																: ''
														)}
														on:click={() => {}}
													>
														<div
															class="w-full h-full items-center text-xs font-normal grid grid-cols-10 gap-0 min-w-0"
														>
															<div class="col-span-1">
																<div
																	class="rounded-full w-2 h-2 {r?.document.success[0]
																		? 'bg-green-400'
																		: 'bg-red-400'}"
																/>
															</div>
															<div class="col-span-5">
																{r?.document.script_path}
															</div>
															<div
																class="whitespace-nowrap col-span-2 !text-tertiary !text-2xs overflow-hidden text-ellipsis flex-shrink text-center"
															>
																{displayDateOnly(new Date(r?.document.created_at[0]))}
															</div>
															<div
																class="whitespace-nowrap col-span-2 !text-tertiary !text-2xs overflow-hidden text-ellipsis flex-shrink text-center"
															>
																<TimeAgo date={r?.document.created_at[0] ?? ''} />
															</div>
														</div>
													</button>
												</svelte:fragment>
											</QuickMenuItem>
										{/each}
									</div>
									{#if selectedItem === undefined}
										select a result to preview
									{:else}
										<div class="w-7/12 overflow-y-scroll">
											<JobPreview
												id={selectedItem?.document?.id[0]}
												workspace={selectedWorkspace}
											/>
										</div>
									{/if}
								{:else}
									<div class="flex w-full justify-center items-center h-96">
										<div class="text-tertiary text-center">
											<div class="text-2xl font-bold">No runs found</div>
											<div class="text-sm">There were no completed runs that match your query</div>
											{#if !$enterpriseLicense}
												<div class="py-6" />

												<Alert title="This is an EE feature" type="warning">
													Full-text search on jobs is only available on EE.
												</Alert>
											{/if}
										</div>
									</div>
								{/if}
							</div>
						{/if}
					</div>
				</div>
			</div>
		</div>
	</Portal>
{/if}

<style>
	.quick-search-input {
		outline: none;
		border: none !important;
		box-shadow: none !important;
	}

	.quick-search-input:focus-visible {
		outline: none !important;
	}
</style>
