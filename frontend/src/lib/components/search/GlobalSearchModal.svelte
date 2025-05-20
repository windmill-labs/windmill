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
	import { clickOutside, displayDateOnly, isMac, sendUserToast } from '$lib/utils'
	import TimeAgo from '../TimeAgo.svelte'
	import {
		AlertTriangle,
		BoxesIcon,
		CalendarIcon,
		Code2Icon,
		Database,
		DollarSignIcon,
		HomeIcon,
		LayoutDashboardIcon,
		Loader2,
		PlayIcon,
		Route,
		Search,
		SearchCode,
		Unplug
	} from 'lucide-svelte'
	import JobPreview from '../runs/JobPreview.svelte'
	import Portal from '$lib/components/Portal.svelte'

	import { twMerge } from 'tailwind-merge'
	import ContentSearchInner from '../ContentSearchInner.svelte'
	import { goto } from '$app/navigation'
	import QuickMenuItem from '../search/QuickMenuItem.svelte'
	import { devopsRole, enterpriseLicense, userStore, workspaceStore } from '$lib/stores'
	import uFuzzy from '@leeoniya/ufuzzy'
	import BarsStaggered from '../icons/BarsStaggered.svelte'
	import { scroll_into_view_if_needed_polyfill } from '../multiselect/utils'
	import { Alert } from '../common'
	import Popover from '../Popover.svelte'
	import Logs from 'lucide-svelte/icons/logs'
	import { AwsIcon, GoogleCloudIcon, KafkaIcon, MqttIcon, NatsIcon } from '../icons'

	let open: boolean = false

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
		shortcutKey?: string
		disabled?: boolean
	}
	let switchModeItems: quickMenuItem[] = [
		{
			search_id: 'switchto:run-search',
			label: 'Search across completed runs' + ($enterpriseLicense ? '' : ' (EE)'),
			action: () => switchMode('runs'),
			shortcutKey: RUNS_PREFIX,
			icon: Search,
			disabled: false
		},
		{
			search_id: 'switchto:content-search',
			label: 'Search scripts/flows/apps based on content',
			action: () => switchMode('content'),
			shortcutKey: CONTENT_SEARCH_PREFIX,
			icon: SearchCode,
			disabled: false
		}
	]

	// These items are searchable but do not appear initially on the menu.
	let hiddenMenuItems = [
		{
			search_id: 'nav:http_routes',
			label: 'Go to HTTP routes',
			action: () => gotoPage('/routes'),
			icon: Route,
			disabled: $userStore?.operator
		},
		{
			search_id: 'nav:web_sockets',
			label: 'Go to WebSockets',
			action: () => gotoPage('/websocket_triggers'),
			icon: Unplug,
			disabled: $userStore?.operator
		},
		{
			search_id: 'nav:postgres_triggers',
			label: 'Go to Postgres triggers',
			action: () => gotoPage('/postgres_triggers'),
			icon: Database,
			disabled: $userStore?.operator
		},
		{
			search_id: 'nav:kafka_triggers',
			label: 'Go to Kafka triggers' + ($enterpriseLicense ? '' : ' (EE)'),
			action: () => gotoPage('/kafka_triggers'),
			icon: KafkaIcon,
			disabled: $userStore?.operator
		},
		{
			search_id: 'nav:nats_triggers',
			label: 'Go to NATS triggers' + ($enterpriseLicense ? '' : ' (EE)'),
			action: () => gotoPage('/nats_triggers'),
			icon: NatsIcon,
			disabled: $userStore?.operator
		},
		{
			search_id: 'nav:sqs_triggers',
			label: 'Go to SQS triggers' + ($enterpriseLicense ? '' : ' (EE)'),
			action: () => gotoPage('/sqs_triggers'),
			icon: AwsIcon,
			disabled: $userStore?.operator
		},
		{
			search_id: 'nav:gcp_pub_sub',
			label: 'Go to GCP Pub/Sub' + ($enterpriseLicense ? '' : ' (EE)'),
			action: () => gotoPage('/gcp_triggers'),
			icon: GoogleCloudIcon,
			disabled: $userStore?.operator
		},
		{
			search_id: 'nav:mqtt_triggers',
			label: 'Go to MQTT triggers',
			action: () => gotoPage('/mqtt_triggers'),
			icon: MqttIcon,
			disabled: $userStore?.operator
		}
	]

	let defaultMenuItems: quickMenuItem[] = [
		{
			search_id: 'nav:home',
			label: 'Go to Home',
			action: () => gotoPage('/'),
			icon: HomeIcon,
			disabled: false
		},
		{
			search_id: 'nav:runs',
			label: 'Go to Runs',
			action: () => gotoPage('/runs'),
			icon: PlayIcon,
			disabled: false
		},
		{
			search_id: 'nav:variables',
			label: 'Go to Variables',
			action: () => gotoPage('/variables'),
			icon: DollarSignIcon,
			disabled: false
		},
		{
			search_id: 'nav:resources',
			label: 'Go to Resources',
			action: () => gotoPage('/resources'),
			icon: BoxesIcon,
			disabled: false
		},
		{
			search_id: 'nav:schedules_triggers',
			label: 'Go to Schedules',
			action: () => gotoPage('/schedules'),
			icon: CalendarIcon,
			disabled: false
		},
		...switchModeItems,
		{
			search_id: 'nav:service_logs',
			label: 'Explore windmill service logs',
			action: () => gotoPage('/service_logs'),
			shortcutKey: LOGS_PREFIX,
			icon: Logs,
			disabled: !$devopsRole
		}
	]

	let defaultMenuItemsWithHidden = [...defaultMenuItems, ...hiddenMenuItems]

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
	let defaultMenuItemAndHiddenLabels = defaultMenuItemsWithHidden.map((item) => item.label)
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

	let queryParseErrors: string[] = []
	let indexMetadata: any = {}

	async function handleSearch() {
		queryParseErrors = []

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
			if (searchTerm === '')
				itemMap['default'] = fuzzyFilter(searchTerm, defaultMenuItems, defaultMenuItemLabels)
			else
				itemMap['default'] = fuzzyFilter(
					searchTerm,
					defaultMenuItemsWithHidden,
					defaultMenuItemAndHiddenLabels
				)
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
			const s = removePrefix(searchTerm, RUNS_PREFIX)
			clearTimeout(debounceTimeout)
			loadingCompletedRuns = true
			debounceTimeout = setTimeout(async () => {
				clearTimeout(debounceTimeout)
				let searchResults
				try {
					searchResults = await IndexSearchService.searchJobsIndex({
						searchQuery: s,
						workspace: $workspaceStore!
					})
					itemMap['runs'] = searchResults.hits
					queryParseErrors = searchResults.query_parse_errors
					indexMetadata = searchResults.index_metadata
				} catch (e) {
					sendUserToast(e.body, true)
				}
				loadingCompletedRuns = false
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
		if ((!isMac() ? event.ctrlKey : event.metaKey) && event.key === 'k') {
			event.preventDefault()
			await openModal()
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
			if (tab != 'logs') {
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

	let mouseMoved: boolean = false
	function handleMouseMove() {
		mouseMoved = true
	}

	onMount(() => {
		window.addEventListener('keydown', handleKeydown)
		window.addEventListener('mousemove', handleMouseMove)
	})

	onDestroy(() => {
		window.removeEventListener('keydown', handleKeydown)
		window.removeEventListener('mousemove', handleMouseMove)
	})

	$: searchTerm, handleSearch()

	function placeholderFromPrefix(text: string): string {
		switch (text) {
			case '':
				return '   Search or type `?` for search options'
			case RUNS_PREFIX:
				return '   Search across completed runs'
			case LOGS_PREFIX:
				return '   Search across completed runs'
			case CONTENT_SEARCH_PREFIX:
				return '   Search flows/scripts/apps by content'
			default:
				return ''
		}
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

	export async function openSearchWithPrefilledText(text?: string) {
		await openModal()
		searchTerm = text ?? searchTerm
		await handleSearch()
	}

	async function focusTextInput() {
		await tick()

		textInput?.focus()
		textInput?.select()

		if (open) {
			if (combinedItems == undefined) {
				combinedItems = await fetchCombinedItems()
				handleSearch()
			}
			selectedItem = selectItem(0)
		}
	}

	async function openModal() {
		open = !open
		focusTextInput()
	}

	function closeModal() {
		open = false
	}

	function maxModalWidth(tab: SearchMode) {
		if (tab === 'runs') {
			return 'max-w-7xl'
		} else {
			return 'max-w-4xl'
		}
	}

	function maxModalHeight(tab: SearchMode) {
		if (tab === 'runs') {
			return ''
		} else if (tab === 'content') {
			return 'max-h-[70vh]'
		} else {
			return 'max-h-[60vh]'
		}
	}
</script>

{#if open}
	<Portal name="global-search">
		<div
			class={twMerge(
				`fixed top-0 bottom-0 left-0 right-0 transition-all duration-50 flex items-start justify-center`,
				' bg-black bg-opacity-40',
				'z-[1100]'
			)}
		>
			<div
				class="{maxModalWidth(tab)} w-full mt-36 bg-surface rounded-lg relative"
				use:clickOutside={false}
				on:click_outside={() => {
					open = false
				}}
			>
				<div class="px-4 py-2 flex flex-row gap-1 items-center border-b">
					<Search size="16" />
					<div class="relative inline-block w-full">
						<input
							id="quickSearchInput"
							bind:this={textInput}
							type="text"
							class="quick-search-input !bg-surface"
							bind:value={searchTerm}
							autocomplete="off"
						/>
						<label
							for="quickSearchInput"
							class="absolute top-1/2 left-2 transform -translate-y-1/2 pointer-events-none text-gray-400 transition-all duration-200 whitespace-pre"
							>{placeholderFromPrefix(searchTerm)}</label
						>
					</div>
					{#if queryParseErrors.length > 0}
						<Popover notClickable placement="bottom-start">
							<AlertTriangle size={16} class="text-yellow-500" />
							<svelte:fragment slot="text">
								Some of your search terms have been ignored because one or more parse errors:<br
								/><br />
								<ul>
									{#each queryParseErrors as msg}
										<li>- {msg}</li>
									{/each}
								</ul>
							</svelte:fragment>
						</Popover>
					{/if}
				</div>
				<div class="overflow-y-auto relative {maxModalHeight(tab)}">
					{#if tab === 'default' || tab === 'switch-mode'}
						{@const items = (itemMap[tab] ?? []).filter((e) =>
							defaultMenuItemsWithHidden.includes(e)
						)}
						{#if items.length > 0}
							<div class={tab === 'switch-mode' ? 'p-2' : 'p-2 border-b'}>
								{#each items as el}
									{#if !el.disabled}
										<QuickMenuItem
											on:select={el?.action}
											on:hover={() => (selectedItem = el)}
											id={el?.search_id}
											hovered={el?.search_id === selectedItem?.search_id}
											label={el?.label}
											icon={el?.icon}
											shortcutKey={el?.shortcutKey}
											bind:mouseMoved
										/>
									{/if}
								{/each}
							</div>
						{/if}
					{/if}

					{#if tab === 'default'}
						<div class="p-2">
							{#if (itemMap[tab] ?? []).filter((e) => (combinedItems ?? []).includes(e)).length > 0}
								<div class="py-2 px-1 text-xs font-semibold text-tertiary">
									Flows/Scripts/Apps
								</div>
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
										bind:mouseMoved
									/>
								{/each}
							{/if}

							{#if (itemMap[tab] ?? []).length === 0}
								<div class="flex w-full justify-center items-center">
									<div class="text-tertiary text-center">
										<div class="text-2xl font-bold">Nothing found</div>
										<div class="text-sm">Tip: press `esc` to quickly clear the search bar</div>
									</div>
								</div>
							{/if}
						</div>
					{:else if tab === 'content'}
						<ContentSearchInner
							search={removePrefix(searchTerm, '#')}
							bind:this={contentSearch}
							on:close={() => {
								closeModal()
							}}
						/>
					{:else if tab === 'logs'}
						<div class="p-2">
							{#if !$devopsRole}
								<Alert title="Service logs are only available to superadmins" type="warning">
									Service logs are only available to superadmins
								</Alert>
							{:else}
								<QuickMenuItem
									on:select={() =>
										gotoPage(
											`/service_logs?query=${encodeURIComponent(removePrefix(searchTerm, '!'))}`
										)}
									id="goto_service_logs_search"
									hovered={true}
									label={searchTerm === '!'
										? 'Explore Windmill service logs'
										: `Search '${removePrefix(searchTerm, '!')}' in Windmill's service logs`}
									icon={searchTerm === '!' ? Logs : Search}
								/>
							{/if}
						</div>
					{:else if tab === 'runs'}
						<div class="flex h-full p-2 divide-x">
							{#if loadingCompletedRuns}
								<div class="flex w-full justify-center items-center h-48">
									<div class="text-tertiary text-center">
										<Loader2 size={34} class="animate-spin" />
									</div>
								</div>
							{:else if itemMap['runs'] && itemMap['runs'].length > 0}
								<div class="w-4/12 overflow-y-auto max-h-[70vh]">
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
											containerClass="rounded-md px-2 py-1 my-2"
											bind:mouseMoved
										>
											<svelte:fragment slot="itemReplacement">
												<div
													class={twMerge(
														`w-full flex flex-row items-center gap-4 transition-all`,
														r?.document.id === selectedItem?.document?.id ? 'bg-surface-hover' : ''
													)}
												>
													<div
														class="rounded-full w-2 h-2 {r?.document.success[0]
															? 'bg-green-400'
															: 'bg-red-400'}"
													></div>
													<div class="flex flex-col gap-2">
														<div class="text-xs"> {r?.document.script_path} </div>
														<div class="flex flex-row gap-2">
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
													</div>
												</div>
											</svelte:fragment>
										</QuickMenuItem>
									{/each}
								</div>
								<div class="w-8/12 max-h-[70vh]">
									{#if selectedItem === undefined}
										Select a result to preview
									{:else}
										<div class="h-[95%] overflow-y-scroll">
											<JobPreview
												id={selectedItem?.document?.id[0]}
												workspace={selectedWorkspace}
											/>
										</div>
									{/if}
									<div class="flex flex-row pt-3 pl-4 items-center text-xs text-secondary">
										{#if indexMetadata.indexed_until}
											<span class="px-2">
												Most recently indexed job was created at <TimeAgo
													agoOnlyIfRecent
													date={indexMetadata.indexed_until || ''}
												/>
											</span>
										{/if}
										{#if indexMetadata.lost_lock_ownership}
											<Popover notClickable placement="top">
												<AlertTriangle size={16} class="text-gray-500" />
												<svelte:fragment slot="text">
													The current indexer is no longer indexing new jobs. This is most likely
													because of an ongoing deployment and indexing will resume once it's
													complete.
												</svelte:fragment>
											</Popover>
										{/if}
									</div>
								</div>
							{:else}
								<div class="flex flex-col h-full w-full justify-center items-center h-48">
									<div class="text-tertiary text-center">
										{#if searchTerm === RUNS_PREFIX}
											<div class="text-2xl font-bold">Enter your search terms</div>
											<div class="text-sm"
												>Start typing to do full-text search across completed runs</div
											>
										{:else}
											<div class="text-2xl font-bold">No runs found</div>
											<div class="text-sm">There were no completed runs that match your query</div>
										{/if}
										<div class="text-sm">
											Note that new runs might take a while to become searchable (by default ~5min)
										</div>
										{#if !$enterpriseLicense}
											<div class="py-6"></div>

											<Alert title="This is an EE feature" type="warning">
												Full-text search on jobs is only available on EE.
											</Alert>
										{/if}
									</div>
									<div class="flex flex-row pt-10 text-xs text-secondary">
										{#if indexMetadata.indexed_until}
											<span class="px-2">
												Most recently indexed job was created at <TimeAgo
													agoOnlyIfRecent
													date={indexMetadata.indexed_until}
												/>
											</span>
										{/if}
										{#if indexMetadata.lost_lock_ownership}
											<Popover notClickable placement="top">
												<AlertTriangle size={16} class="text-gray-500" />
												<svelte:fragment slot="text">
													The current indexer is no longer indexing new jobs. This is most likely
													because of an ongoing deployment and indexing will resume once it's
													complete.
												</svelte:fragment>
											</Popover>
										{/if}
									</div>
								</div>
							{/if}
						</div>
					{/if}
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
