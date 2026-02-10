<script lang="ts">
	import { onDestroy, onMount, tick, untrack } from 'svelte'
	import {
		AppService,
		FlowService,
		ScriptService,
		type Flow,
		type ListableApp,
		type ListableRawApp,
		type Script,
		type SearchJobsIndexResponse
	} from '$lib/gen'
	import { clickOutside, isMac, scroll_into_view_if_needed_polyfill } from '$lib/utils'
	import {
		AlertTriangle,
		BoxesIcon,
		CalendarIcon,
		Code2Icon,
		Database,
		DollarSignIcon,
		HomeIcon,
		LayoutDashboardIcon,
		MailIcon,
		PlayIcon,
		Route,
		Search,
		SearchCode,
		Unplug,
		WandSparkles
	} from 'lucide-svelte'
	import Portal from '$lib/components/Portal.svelte'

	import { twMerge } from 'tailwind-merge'
	import ContentSearchInner from '../ContentSearchInner.svelte'
	import { goto } from '$app/navigation'
	import QuickMenuItem from '../search/QuickMenuItem.svelte'
	import { devopsRole, enterpriseLicense, userStore, workspaceStore } from '$lib/stores'
	import uFuzzy from '@leeoniya/ufuzzy'
	import BarsStaggered from '../icons/BarsStaggered.svelte'
	import { Alert } from '../common'
	import Popover from '../Popover.svelte'
	import Logs from 'lucide-svelte/icons/logs'
	import { AwsIcon, GoogleCloudIcon, KafkaIcon, MqttIcon, NatsIcon } from '../icons'
	import RunsSearch from './RunsSearch.svelte'
	import AskAiButton from '../copilot/AskAiButton.svelte'

	let open: boolean = $state(false)

	let searchTerm: string = $state('')
	let textInput: HTMLInputElement | undefined = $state()
	let selectedWorkspace: string | undefined = $state(undefined)
	let contentSearch: ContentSearchInner | undefined = $state(undefined)

	const RUNS_PREFIX = '>'
	const LOGS_PREFIX = '!'
	const CONTENT_SEARCH_PREFIX = '#'
	const SWITCH_MODE_PREFIX = '?'

	type SearchMode = 'default' | 'switch-mode' | 'runs' | 'content' | 'logs'

	let tab: SearchMode = $state('default')

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
			label: 'Search across completed runs' + (!$enterpriseLicense ? '' : ' (EE)'),
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
			action: (newtab: boolean = false) => gotoPage('/routes', newtab),
			icon: Route,
			disabled: $userStore?.operator
		},
		{
			search_id: 'nav:web_sockets',
			label: 'Go to WebSockets',
			action: (newtab: boolean = false) => gotoPage('/websocket_triggers', newtab),
			icon: Unplug,
			disabled: $userStore?.operator
		},
		{
			search_id: 'nav:postgres_triggers',
			label: 'Go to Postgres triggers',
			action: (newtab: boolean = false) => gotoPage('/postgres_triggers', newtab),
			icon: Database,
			disabled: $userStore?.operator
		},
		{
			search_id: 'nav:kafka_triggers',
			label: 'Go to Kafka triggers' + (!$enterpriseLicense ? '' : ' (EE)'),
			action: (newtab: boolean = false) => gotoPage('/kafka_triggers', newtab),
			icon: KafkaIcon,
			disabled: $userStore?.operator
		},
		{
			search_id: 'nav:nats_triggers',
			label: 'Go to NATS triggers' + (!$enterpriseLicense ? '' : ' (EE)'),
			action: (newtab: boolean = false) => gotoPage('/nats_triggers', newtab),
			icon: NatsIcon,
			disabled: $userStore?.operator
		},
		{
			search_id: 'nav:sqs_triggers',
			label: 'Go to SQS triggers' + (!$enterpriseLicense ? '' : ' (EE)'),
			action: (newtab: boolean = false) => gotoPage('/sqs_triggers', newtab),
			icon: AwsIcon,
			disabled: $userStore?.operator
		},
		{
			search_id: 'nav:gcp_pub_sub',
			label: 'Go to GCP Pub/Sub' + (!$enterpriseLicense ? '' : ' (EE)'),
			action: (newtab: boolean = false) => gotoPage('/gcp_triggers', newtab),
			icon: GoogleCloudIcon,
			disabled: $userStore?.operator
		},
		{
			search_id: 'nav:mqtt_triggers',
			label: 'Go to MQTT triggers',
			action: (newtab: boolean = false) => gotoPage('/mqtt_triggers', newtab),
			icon: MqttIcon,
			disabled: $userStore?.operator
		},
		{
			search_id: 'nav:email_triggers',
			label: 'Go to Email triggers',
			action: (newtab: boolean = false) => gotoPage('/email_triggers', newtab),
			icon: MailIcon,
			disabled: $userStore?.operator
		}
	]

	let defaultMenuItems: quickMenuItem[] = [
		{
			search_id: 'nav:home',
			label: 'Go to Home',
			action: (newtab: boolean = false) => gotoPage('/', newtab),
			icon: HomeIcon,
			disabled: false
		},
		{
			search_id: 'nav:runs',
			label: 'Go to Runs',
			action: (newtab: boolean = false) => gotoPage('/runs', newtab),
			icon: PlayIcon,
			disabled: false
		},
		{
			search_id: 'nav:variables',
			label: 'Go to Variables',
			action: (newtab: boolean = false) => gotoPage('/variables', newtab),
			icon: DollarSignIcon,
			disabled: false
		},
		{
			search_id: 'nav:resources',
			label: 'Go to Resources',
			action: (newtab: boolean = false) => gotoPage('/resources', newtab),
			icon: BoxesIcon,
			disabled: false
		},
		{
			search_id: 'nav:schedules_triggers',
			label: 'Go to Schedules',
			action: (newtab: boolean = false) => gotoPage('/schedules', newtab),
			icon: CalendarIcon,
			disabled: false
		},
		...switchModeItems,
		{
			search_id: 'nav:service_logs',
			label: 'Explore windmill service logs',
			action: (newtab: boolean = false) => gotoPage('/service_logs', newtab),
			shortcutKey: LOGS_PREFIX,
			icon: Logs,
			disabled: !$devopsRole
		}
	]

	let defaultMenuItemsWithHidden = [...defaultMenuItems, ...hiddenMenuItems]

	let itemMap = $state({
		default: defaultMenuItems as any[],
		'switch-mode': switchModeItems,
		runs: [] as any[],
		content: [] as any[],
		logs: [] as any[]
	})

	$effect(() => {
		tab === 'content' && contentSearch?.open()
	})

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
		textInput?.focus()
	}

	function removePrefix(str: string, prefix: string): string {
		if (str.startsWith(prefix)) {
			return str.substring(prefix.length)
		}
		return str
	}

	let opts: uFuzzy.Options = {}

	let uf = new uFuzzy(opts)
	// let defaultMenuItemLabels = defaultMenuItems.map((item) => item.label)
	let defaultMenuItemAndHiddenLabels = defaultMenuItemsWithHidden.map((item) => item.label)
	let switchModeItemLabels = switchModeItems.map((item) => item.label)
	let askAiButton: AskAiButton | undefined = $state()

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

	let queryParseErrors: string[] = $state([])

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
			if (searchTerm === '') itemMap['default'] = defaultMenuItems
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
			itemMap['default'] = itemMap['default'].filter((e) => !e.disabled)
		}
		if (tab === 'switch-mode') {
			itemMap['switch-mode'] = fuzzyFilter(
				removePrefix(searchTerm, SWITCH_MODE_PREFIX),
				switchModeItems,
				switchModeItemLabels
			)
		}
		if (tab === 'runs') {
			await tick()
			runsSearch?.handleRunSearch(removePrefix(searchTerm, RUNS_PREFIX))
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

	let selectedItem: any = $state()

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
		textInput?.focus()
	}

	function gotoWindmillItemPage(e: TableAny, newtab: boolean = false) {
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
		gotoPage(path, newtab)
	}

	function gotoPage(path: string, newtab: boolean = false) {
		searchTerm = ''
		if (!newtab) {
			open = false
			goto(path)
		} else {
			window.open(path, '_blank')
		}
	}

	let mouseMoved: boolean = $state(false)
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

	$effect(() => {
		searchTerm
		untrack(() => handleSearch())
	})

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

	let combinedItems: TableAny[] | undefined = $state(undefined)

	async function fetchCombinedItems() {
		const scripts = await ScriptService.listScripts({
			workspace: $workspaceStore!,
			withoutDescription: true
		})
		const flows = await FlowService.listFlows({
			workspace: $workspaceStore!,
			withoutDescription: true
		})
		const apps = await AppService.listApps({ workspace: $workspaceStore! })

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

	let runsSearch: RunsSearch | undefined = $state()
	let runSearchRemainingCount: number | undefined = $state(undefined)
	let runSearchTotalCount: number | undefined = $state(undefined)
	let indexMetadata: SearchJobsIndexResponse['index_metadata'] = $state(undefined)
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
				use:clickOutside={{
					capture: false,
					onClickOutside: () => {
						open = false
					}
				}}
			>
				<div class="px-4 py-2 flex flex-row gap-1 items-center border-b">
					<Search size="16" />
					<div class="relative inline-block flex-1">
						<input
							id="quickSearchInput"
							bind:this={textInput}
							type="text"
							class="no-default-style !bg-transparent !border-none w-full !ring-0 !text-sm"
							bind:value={searchTerm}
							autocomplete="off"
						/>
						<label
							for="quickSearchInput"
							class="absolute top-1/2 left-2 transform -translate-y-1/2 pointer-events-none text-gray-400 transition-all duration-200 whitespace-pre text-xs"
						>
							{placeholderFromPrefix(searchTerm)}
						</label>
					</div>
					{#if (itemMap[tab] ?? []).length === 0 && searchTerm.length > 0}
						<AskAiButton
							bind:this={askAiButton}
							label="Ask AI"
							initialInput={searchTerm}
							onClick={() => {
								closeModal()
							}}
						/>
					{/if}
					{#if queryParseErrors.length > 0}
						<Popover notClickable placement="bottom-start">
							<AlertTriangle size={16} class="text-yellow-500" />
							{#snippet text()}
								Some of your search terms have been ignored because one or more parse errors:<br
								/><br />
								<ul>
									{#each queryParseErrors as msg}
										<li>- {msg}</li>
									{/each}
								</ul>
							{/snippet}
						</Popover>
					{/if}
				</div>
				<div class="overflow-y-auto relative {maxModalHeight(tab)}">
					{#if tab === 'default' || tab === 'switch-mode'}
						{@const items = (itemMap[tab] ?? []).filter((e) =>
							defaultMenuItemsWithHidden.some((x) => e.search_id === x.search_id)
						)}
						{#if items.length > 0}
							<div
								class={tab === 'switch-mode' || itemMap[tab].length === items.length
									? 'p-2'
									: 'p-2 border-b'}
							>
								{#each items as el}
									<QuickMenuItem
										onselect={(shift) => el?.action(shift)}
										onhover={() => (selectedItem = el)}
										id={el?.search_id}
										hovered={el?.search_id === selectedItem?.search_id}
										label={el?.label}
										icon={el?.icon}
										shortcutKey={el?.shortcutKey}
										bind:mouseMoved
									/>
								{/each}
							</div>
						{/if}
					{/if}

					{#if tab === 'default'}
						{#if (itemMap[tab] ?? []).filter((e) => (combinedItems ?? []).includes(e)).length > 0}
							<div class="p-2">
								<div class="py-2 px-1 text-xs font-semibold text-primary"> Flows/Scripts/Apps </div>
								{#each (itemMap[tab] ?? []).filter((e) => (combinedItems ?? []).includes(e)) as el}
									<QuickMenuItem
										onselect={(shift) => {
											gotoWindmillItemPage(el, shift)
										}}
										onhover={() => (selectedItem = el)}
										id={el?.search_id}
										hovered={el?.path === selectedItem?.path}
										label={(el.summary ? `${el.summary} - ` : '') +
											el.path +
											(el.starred ? ' ★' : '')}
										icon={iconForWindmillItem(el.type)}
										bind:mouseMoved
									/>
								{/each}
							</div>
						{/if}

						{#if (itemMap[tab] ?? []).length === 0}
							<div class="p-2">
								<QuickMenuItem
									onselect={() => {
										askAiButton?.onClick()
									}}
									id={'ai:no-results-ask-ai'}
									hovered={true}
									label={`Try asking \`${searchTerm}\` to AI`}
									icon={WandSparkles}
									bind:mouseMoved
								/>
								<div class="flex w-full justify-center items-center">
									<div class="text-primary text-center">
										<div class="pt-1 text-sm">Tip: press `esc` to quickly clear the search bar</div>
									</div>
								</div>
							</div>
						{/if}
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
									onselect={() =>
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
						<RunsSearch
							bind:queryParseErrors
							bind:this={runsSearch}
							bind:selectedItem
							bind:selectedWorkspace
							bind:mouseMoved
							bind:loadedRuns={itemMap['runs']}
							bind:open
							{selectItem}
							searchTerm={removePrefix(searchTerm, RUNS_PREFIX)}
							bind:runSearchRemainingCount
							bind:runSearchTotalCount
							bind:indexMetadata
						/>
					{/if}
				</div>
			</div>
		</div>
	</Portal>
{/if}
