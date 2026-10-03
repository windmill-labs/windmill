<script lang="ts">
	import { IndexSearchService, ServiceLogsService, type LogSearchHit } from '$lib/gen'

	import TimeframeSelect, {
		serviceLogsTimeframes,
		useUrlSyncedTimeframe
	} from './runs/TimeframeSelect.svelte'
	import CalendarPicker from './common/calendarPicker/CalendarPicker.svelte'
	import LogViewer from './LogViewer.svelte'
	import Toggle from './Toggle.svelte'
	import TextInput from './text_input/TextInput.svelte'
	import Popover from './Popover.svelte'
	import { sendUserToast } from '$lib/toast'
	import { onDestroy, tick, type Snippet } from 'svelte'
	import { fade } from 'svelte/transition'
	import {
		AlertTriangle,
		ArrowDown,
		ChevronLeft,
		ChevronRight,
		ClipboardCopy,
		Loader2,
		ScrollText,
		Search,
		SearchX
	} from 'lucide-svelte'
	import { copyToClipboard, scroll_into_view_if_needed_polyfill, truncateRev } from '$lib/utils'
	import LogSnippetViewer from './LogSnippetViewer.svelte'
	import { Alert, Button, Drawer, DrawerContent, EmptyState, Skeleton } from './common'
	import { AnsiUp } from 'ansi_up'
	import SplitPanesOrColumnOnMobile from './splitPanes/SplitPanesOrColumnOnMobile.svelte'
	import Select from './select/Select.svelte'
	import { goto } from '$lib/navigation'
	import { page } from '$app/state'
	import { watch } from 'runed'

	interface Props {
		searchTerm: string
		tagLabel?: string
		/** Rendered at the start of the toolbar row, so the page title shares it. */
		title?: Snippet
	}

	let { searchTerm = $bindable(), tagLabel, title }: Props = $props()

	let queryParseErrors: string[] | undefined = $state(undefined)

	let minTs: undefined | string = $state(undefined)
	let maxTs: undefined | string = $state(undefined)

	// let lastSeen: undefined | string = undefined

	let withError = $state(false)
	let autoRefresh = $state(true)
	let loading = $state(false)

	type LogFile = {
		ts: number
		file_path: string
		ok_lines: number
		err_lines: number
		json_fmt: boolean
	}

	type ByHostname = Record<string, LogFile[]>
	type ByWorkerGroup = Record<string, ByHostname>
	type ByMode = Record<string, ByWorkerGroup>

	let timeout: number | undefined = $state(undefined)
	const REFRESH_OVERLAP_MS = 2 * 60 * 1000

	let allLogs: ByMode | undefined = $state(undefined)

	let _timeframe = useUrlSyncedTimeframe(serviceLogsTimeframes)
	let timeframe = $derived(_timeframe.timeframe)

	let [minTsManual, maxTsManual] = $derived(
		timeframe.type === 'manual' ? [timeframe.minTs ?? undefined, timeframe.maxTs ?? undefined] : []
	)

	let upTo: undefined | string = $state(undefined)
	let upToIsLatest = $state(true)

	function getAllLogs(queryMinTs: string | undefined, queryMaxTs: string | undefined) {
		timeout && clearTimeout(timeout)
		loading = true
		allLogs = allLogs ?? {}
		ServiceLogsService.listLogFiles({ withError, before: queryMaxTs, after: queryMinTs })
			.then((res) => {
				loading = false

				let minTsN: number | undefined = undefined
				let maxTsN: number | undefined = undefined
				if (minTsManual) {
					minTsN = new Date(minTsManual).getTime()
					Object.values(allLogs ?? {}).forEach((mode) => {
						Object.values(mode).forEach((wg) => {
							Object.keys(wg).forEach((key) => {
								wg[key] = wg[key].filter(
									(x) => !minTsManual || x.ts >= new Date(minTsManual).getTime()
								)
							})
						})
					})
				}

				res.reverse().forEach((log) => {
					let ts = new Date(log.log_ts + 'Z').getTime()
					if (minTsN == undefined || ts < minTsN) {
						minTsN = ts
					}
					if (maxTsN == undefined || ts > maxTsN) {
						maxTsN = ts
					}
					if (allLogs == undefined) {
						allLogs = {}
					}
					if (!allLogs[log.mode]) {
						allLogs[log.mode] = {}
					}
					const wg = log.worker_group ?? ''
					if (!allLogs[log.mode][wg]) {
						allLogs[log.mode][wg] = {}
					}
					const hn = log.hostname ?? ''
					if (!allLogs[log.mode][wg][hn]) {
						allLogs[log.mode][wg][hn] = []
					}
					const files = allLogs[log.mode][wg][hn]
					// Refreshes overlap the previous window, so a file can come back twice.
					if (files.some((f) => f.file_path === log.file_path)) {
						return
					}
					files.push({
						ts: ts,
						file_path: log.file_path,
						ok_lines: log.ok_lines ?? 1,
						err_lines: log.err_lines ?? 0,
						json_fmt: log.json_fmt
					})
					// A late file can be older than one already listed; readers take the
					// last entries as the newest.
					if (files.length > 1 && files[files.length - 2].ts > ts) {
						files.sort((a, b) => a.ts - b.ts)
					}
				})

				Object.values(allLogs ?? {}).forEach((mode) => {
					Object.values(mode).forEach((wg) => {
						Object.keys(wg).forEach((key) => {
							wg[key] = wg[key].filter(
								(x) => !minTsManual || x.ts >= new Date(minTsManual).getTime()
							)
						})
					})
				})

				loading = false
				if (minTs == undefined) {
					minTs = minTsN ? new Date(minTsN).toISOString() : undefined
				}
				if (maxTsN) {
					maxTs = new Date(maxTsN).toISOString()
				}
				if (upToIsLatest && selected) {
					upTo = getLatestUpTo(selected)
				}
				if (autoRefresh && searchTerm === '' && !maxTsManual) {
					timeout = setTimeout(() => {
						if (searchTerm !== '') return
						// Each host's file for a minute is written when that host compacts it, so
						// files for a minute already seen elsewhere keep landing for a while after.
						let refreshFrom = maxTs
							? new Date(new Date(maxTs).getTime() - REFRESH_OVERLAP_MS)
							: undefined
						getAllLogs(refreshFrom?.toISOString(), undefined)
					}, 5000)
				}
			})
			.catch((e) => {
				// Only an API error has a body; a network failure or an exception in the
				// handler above carries its cause in `message`.
				sendUserToast('Failed to load service logs: ' + (e?.body ?? e?.message ?? e), true)
				console.error(e)
				loading = false
				autoRefresh = false
			})
	}

	type Selected = { mode: string; workerGroup: string; hostname: string }
	// Servers and indexers have no worker group, so the URL omits it for them.
	let initialSelected =
		page.url.searchParams.get('mode') && page.url.searchParams.get('hostname')
			? {
					mode: page.url.searchParams.get('mode')!,
					workerGroup: page.url.searchParams.get('workerGroup') ?? '',
					hostname: page.url.searchParams.get('hostname')!
				}
			: undefined
	let selected: Selected | undefined = $state(initialSelected)

	let logsContent: Record<string, { content?: string; error?: string }> = $state({})
	// Files are fetched from the render path, which runs again before a fetch returns.
	const fetchingFiles = new Set<string>()

	/** Resolves to true when this call fetched the file, false when it was already there. */
	// File names are per-minute timestamps, the same on every host, so a file is only
	// identified by its host and name together.
	function fileKey(hostname: string, path: string): string {
		return `${hostname}/${path}`
	}

	export async function getLogFile(hostname: string, path: string): Promise<boolean> {
		const key = fileKey(hostname, path)
		if (logsContent[key] || fetchingFiles.has(key)) {
			return false
		}
		fetchingFiles.add(key)
		try {
			const res = await ServiceLogsService.getLogFile({ path: key })
			logsContent[key] = { content: res }
		} catch (e) {
			logsContent[key] = { error: `${e.message}: ${e.body}` }
		} finally {
			fetchingFiles.delete(key)
		}
		return true
	}

	function getLogs(selected: Selected, upTo: string | undefined) {
		if (!selected) {
			return []
		}
		let logs = allLogs?.[selected.mode]?.[selected.workerGroup]?.[selected.hostname]
		if (!logs) {
			return []
		}
		if (upTo) {
			let upToN = new Date(upTo).getTime()
			let nlogs = logs.filter((x) => x.ts <= upToN)
			logs = nlogs.slice(nlogs.length - 5, undefined)

			getFiles(
				selected.hostname,
				logs.map((x) => x.file_path)
			)
		}
		return logs
	}

	async function getFiles(hostname: string, logs: string[]) {
		const fetched = await Promise.all(logs.map((x) => getLogFile(hostname, x)))
		if (fetched.some(Boolean)) {
			await tick()
			onLogsAppended()
		} else {
			// Every file was cached: `jumpToEnd` already landed, and the next file that
			// arrives extends the view, so it should animate.
			jumpPending = false
		}
	}

	function getLatestUpTo(selected: Selected): any {
		if (!selected) {
			return undefined
		}
		let logs = allLogs?.[selected.mode]?.[selected.workerGroup]?.[selected.hostname]
		if (!logs) {
			return undefined
		}
		return logs[logs.length - 1]?.ts
	}

	// Within this distance of the end, the reader is following the tail.
	const AT_END_PX = 24
	let logPane: HTMLDivElement | undefined = $state()
	let followTail = $state(true)
	let showScrollToEnd = $state(false)
	let hasNewLogs = $state(false)
	// Set by an explicit navigation (host, time range); the next content lands at the end
	// without animation, since it replaces the view rather than extending it.
	let jumpPending = true
	// A smooth scroll emits scroll events for its whole duration, all short of the end;
	// until it lands, those are ours and must not read as the reader leaving the tail.
	let autoScrollUntil = 0

	function distanceToEnd(): number {
		return logPane ? logPane.scrollHeight - logPane.scrollTop - logPane.clientHeight : 0
	}

	function scrollToEnd(smooth: boolean) {
		if (!logPane) return
		autoScrollUntil = smooth ? Date.now() + 1000 : 0
		logPane.scrollTo({ top: logPane.scrollHeight, behavior: smooth ? 'smooth' : 'auto' })
		followTail = true
		hasNewLogs = false
		showScrollToEnd = false
	}

	async function jumpToEnd() {
		jumpPending = true
		followTail = true
		hasNewLogs = false
		await tick()
		scrollToEnd(false)
	}

	function onLogsAppended() {
		if (jumpPending) {
			jumpPending = false
			scrollToEnd(false)
		} else if (followTail) {
			scrollToEnd(true)
		} else {
			// Older files fetched while browsing back in time are not news.
			if (upToIsLatest) hasNewLogs = true
			showScrollToEnd = distanceToEnd() > AT_END_PX
		}
	}

	let lastScrollTop = 0
	let lastScrollHeight = 0

	function onLogPaneScroll() {
		const atEnd = distanceToEnd() <= AT_END_PX
		const movedUp = (logPane?.scrollTop ?? 0) < lastScrollTop
		const contentChanged = (logPane?.scrollHeight ?? 0) !== lastScrollHeight
		lastScrollTop = logPane?.scrollTop ?? 0
		lastScrollHeight = logPane?.scrollHeight ?? 0
		// Files added or dropped make the browser shift the scroll position to keep the
		// visible lines in place. That is not the reader moving, so it must not change
		// whether they follow the tail.
		if (contentChanged) {
			if (!followTail) showScrollToEnd = !atEnd
			return
		}
		// An automatic scroll only moves down, so moving up during one is the reader.
		if (Date.now() < autoScrollUntil && !movedUp) {
			if (atEnd) autoScrollUntil = 0
			return
		}
		autoScrollUntil = 0
		followTail = atEnd
		showScrollToEnd = !atEnd
		if (atEnd) hasNewLogs = false
		// A jump whose files were all cached never reaches `onLogsAppended`; once the
		// reader leaves the end, a later refresh must not carry it out.
		else jumpPending = false
	}

	onDestroy(() => {
		timeout && clearTimeout(timeout)
	})

	function processLogWithJsonFmt(log: string | undefined, jsonFmt: boolean): string {
		if (!log) {
			return ''
		}
		if (!jsonFmt) {
			return log
		}
		try {
			let res = ''
			log.split('\n').forEach((line) => {
				// A file can hold both formats: the ones written before the layer
				// switched to JSON, and panics or subprocess output that was never
				// JSON to begin with. Those lines pass through as they are rather
				// than being dropped, which would render the file blank.
				let obj: any = undefined
				if (line.startsWith('{') && line.endsWith('}')) {
					try {
						obj = JSON.parse(line)
					} catch {
						obj = undefined
					}
				}
				if (obj === null || typeof obj !== 'object') {
					res += line + '\n'
				} else {
					let nl = ''
					if (obj['timestamp']) {
						nl += obj['timestamp'] + ' '
					}
					if (obj['level']) {
						let lvl = obj['level']
						if (lvl == 'ERROR') {
							nl += '\x1b[31mERROR\x1b[0m '
						} else if (lvl == 'INFO') {
							nl += '\x1b[32mINFO\x1b[0m '
						} else {
							nl += obj['level'] + ' '
						}
					}
					if (obj['message']) {
						nl += obj['message'] + ' '
					}
					delete obj['timestamp']
					delete obj['level']
					delete obj['message']
					Object.keys(obj).forEach((key) => {
						nl +=
							key + '=' + (typeof obj[key] == 'object' ? JSON.stringify(obj[key]) : obj[key]) + ' '
					})
					res += nl + '\n'
				}
			})

			return res
		} catch (e) {
			return log
		}
	}

	// A hit is one log line with its fields already separated, so rendering it is
	// formatting rather than parsing — there is no JSON to prettify and no
	// snippet to highlight.
	function renderHit(hit: LogSearchHit): string {
		const level =
			hit.level === 'ERROR'
				? '\x1b[31mERROR\x1b[0m'
				: hit.level === 'WARN'
					? '\x1b[33mWARN\x1b[0m'
					: hit.level === 'INFO'
						? '\x1b[32mINFO\x1b[0m'
						: hit.level
		return [hit.ts, level, hit.message, hit.target ? `target=${hit.target}` : '']
			.filter(Boolean)
			.join(' ')
	}

	let logs: any = $state()

	let debounceTimeout: number | undefined = undefined
	const debouncePeriod: number = 400
	let loadingLogs = $state(false)
	let loadingLogCounts = $state(false)

	let countsPerHost: any = $state()
	let sumOtherDocCount: number = $state(0)
	let searchError: string | undefined = $state(undefined)

	// `searchLogs` runs on every refresh too, so leaving a search is detected as a
	// transition rather than on each empty-term call.
	let searching = false

	// A search stops the refresh poll and replaces the files with hits.
	function leaveSearch() {
		if (autoRefresh && !maxTsManual && maxTs) {
			getAllLogs(new Date(new Date(maxTs).getTime() - REFRESH_OVERLAP_MS).toISOString(), undefined)
		}
		jumpToEnd()
	}

	async function searchLogs(
		searchTerm: string,
		selected: Selected | undefined,
		minTs: string | undefined,
		maxTs: string | undefined,
		allLogs: ByMode | undefined
	) {
		const params = new URLSearchParams()
		if (searchTerm) params.set('searchTerm', searchTerm)
		if (selected?.mode) params.set('mode', selected.mode)
		if (selected?.workerGroup) params.set('workerGroup', selected.workerGroup)
		if (selected?.hostname) params.set('hostname', selected.hostname)
		// Only syncs the URL: a navigation resets focus by default, which would pull it off
		// the host button or search field the reader is using.
		goto(`?${params.toString()}`, { keepFocus: true, noScroll: true })
		if (searchTerm.trim() === '') {
			debounceTimeout && clearTimeout(debounceTimeout)
			logs = undefined
			countsPerHost = undefined
			sumOtherDocCount = 0
			loadingLogs = false
			loadingLogCounts = false
			searchError = undefined
			if (searching) {
				searching = false
				leaveSearch()
			}
			return
		}
		searching = true
		timeout && clearTimeout(timeout)

		loadingLogCounts = true
		loadingLogs = true
		debounceTimeout && clearTimeout(debounceTimeout)
		debounceTimeout = setTimeout(async () => {
			searchError = undefined
			try {
				if (allLogs) {
					const countLogsResponse = await IndexSearchService.countSearchLogsIndex({
						searchQuery: searchTerm,
						minTs,
						maxTs
					})
					const res = (countLogsResponse.count_per_host as any)['count_per_host']
					const buckets = res['buckets']
					sumOtherDocCount = res['sum_other_doc_count']
					countsPerHost = new Map(buckets.map(({ key, doc_count }) => [key, doc_count]))
					countsPerHost = buckets.reduce(
						(acc: any, { key, doc_count }) => {
							acc[key] = { doc_count }
							return acc
						},
						{} as Record<string, number>
					)
					queryParseErrors = countLogsResponse.query_parse_errors ?? []
				}

				if (selected) {
					logs = await IndexSearchService.searchLogsIndex({
						searchQuery: searchTerm,
						mode: selected.mode,
						workerGroup: selected.workerGroup != '' ? selected.workerGroup : undefined,
						hostname: selected.hostname,
						minTs,
						maxTs
					})
				}
			} catch (e) {
				const message = e?.body ?? e?.message ?? 'Unknown error'
				searchError = message
				// Drop any results from a previous successful search so the error
				// isn't shown alongside stale matches/counts for the old query.
				logs = undefined
				countsPerHost = undefined
				sumOtherDocCount = 0
				console.error(e)
			} finally {
				loadingLogs = false
				loadingLogCounts = false
			}
		}, debouncePeriod)
	}

	const ansi_up = new AnsiUp()
	ansi_up.use_classes = true

	let logDrawer: Drawer | undefined = $state(undefined)
	let logDrawerOpen: boolean = $state(false)
	let content: string = $state('')
	let hitLineNumber: number | undefined = $state(undefined)

	async function seeLogContext(
		lineNumber: number,
		path: string,
		hostname: string,
		jsonFmt: boolean
	) {
		const res = await ServiceLogsService.getLogFile({ path: `${hostname}/${path}` })

		// Prettify first: it emits its own ANSI for the level, which converting
		// beforehand would leave in the output as literal escapes.
		content = ansi_up.ansi_to_html(processLogWithJsonFmt(res, jsonFmt))
		hitLineNumber = lineNumber
		logDrawerOpen = true

		await tick()
		let el = document.getElementById(`log-line-${lineNumber}`)
		if (el) scroll_into_view_if_needed_polyfill(el, false)
	}

	function allLogsOrQueryResults(allLogs: ByMode, countsPerHost: any): ByMode {
		if (countsPerHost == undefined) {
			return allLogs
		}
		let ret = {}

		for (const hk of Object.keys(countsPerHost)) {
			let u = hk.split(',')
			let [mode, wg, hn] = [u[0], u[1], u[2]]

			if (!ret[mode]) {
				ret[mode] = {}
			}
			if (!ret[mode][wg]) {
				ret[mode][wg] = {}
			}
			if (!ret[mode][wg][hn]) {
				ret[mode][wg][hn] = []
			}
		}

		return ret
	}

	function getSelectItems(
		allLogs: ByMode,
		countsPerHost: any
	): { label: string; value: Selected }[] {
		return Object.entries(allLogsOrQueryResults(allLogs, countsPerHost)).flatMap(([mode, o1]) =>
			Object.entries(o1).flatMap(([wg, o2]) =>
				Object.keys(o2).map((hn) => ({
					label: hn,
					value: { mode, workerGroup: wg, hostname: hn }
				}))
			)
		)
	}

	function formatTime(ts: string | number): string {
		return new Date(ts).toLocaleTimeString([], {
			day: '2-digit',
			month: '2-digit',
			hour: '2-digit',
			minute: '2-digit'
		})
	}

	// Time of day only when both ends fall on the same day, so the axis fits a narrow pane.
	function formatAxisTime(ts: string, other: string): string {
		const sameDay = new Date(ts).toDateString() === new Date(other).toDateString()
		return sameDay
			? new Date(ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
			: formatTime(ts)
	}

	// The API returns hosts in log order, which reshuffles the list on every refresh.
	function sortedEntries<T>(o: Record<string, T>): [string, T][] {
		return Object.entries(o).sort(([a], [b]) => a.localeCompare(b))
	}

	function modeLabel(mode: string, plural: boolean): string {
		return mode.charAt(0).toUpperCase() + mode.slice(1) + (plural ? 's' : '')
	}

	function isSelected(mode: string, wg: string, hn: string): boolean {
		return selected?.mode === mode && selected?.workerGroup === wg && selected?.hostname === hn
	}

	function selectHost(mode: string, wg: string, hn: string) {
		selected = { mode, workerGroup: wg, hostname: hn }
		upToIsLatest = true
		upTo = getLatestUpTo(selected)
		jumpToEnd()
	}

	function visibleHosts(mode: string, wg: string, hosts: ByHostname): [string, LogFile[]][] {
		return Object.entries(hosts).filter(([hn]) => {
			if (isSelected(mode, wg, hn)) return true
			const count = countsPerHost?.[`${mode},${wg},${hn}`]?.doc_count
			return !countsPerHost || (count != undefined && count > 0)
		})
	}

	function shiftUpTo(minutes: number) {
		if (!upTo) return
		upToIsLatest = false
		upTo = new Date(new Date(upTo).getTime() + minutes * 60 * 1000).toISOString()
		jumpToEnd()
	}

	watch(
		() => timeframe,
		() => {
			const ts = timeframe.computeMinMax()
			minTs = undefined
			maxTs = undefined
			allLogs = undefined
			getAllLogs(ts.minTs ?? undefined, ts.maxTs ?? undefined)
		}
	)
	watch(
		() => [searchTerm, selected, timeframe, allLogs],
		() => {
			searchLogs(searchTerm, selected, minTsManual, maxTsManual, allLogs)
		}
	)
</script>

<Drawer bind:this={logDrawer} bind:open={logDrawerOpen} size="1400px">
	<DrawerContent title="Log context" on:close={logDrawer.closeDrawer}>
		{#snippet actions()}
			<Button
				onClick={() => copyToClipboard(content)}
				variant="default"
				unifiedSize="md"
				startIcon={{ icon: ClipboardCopy }}
			>
				Copy to clipboard
			</Button>
		{/snippet}
		<pre
			class="w-fit min-w-full rounded-md border border-border-light bg-surface-secondary p-2 font-mono text-2xs text-primary whitespace-pre"
			>{#each content.split('\n') as line, index}<div
					id={`log-line-${index}`}
					class={index === hitLineNumber ? 'bg-surface-accent-selected' : ''}>{@html line}</div
				>{/each}</pre
		>
	</DrawerContent>
</Drawer>

<div class="flex flex-col grow min-h-0 min-w-0 gap-2 pb-2">
	<div class="flex flex-wrap items-center gap-x-4 gap-y-2 pt-4 pb-2">
		{@render title?.()}
		<div class="relative grow min-w-64">
			<Search
				size={16}
				class="absolute left-2 top-1/2 -translate-y-1/2 text-hint pointer-events-none z-10"
			/>
			<TextInput
				bind:value={searchTerm}
				size="md"
				class="pl-8 pr-8"
				inputProps={{
					id: 'quickSearchInput',
					placeholder: 'Search service logs',
					autocomplete: 'off',
					autofocus: true
				}}
			/>
			{#if searchTerm !== '' && queryParseErrors && queryParseErrors.length > 0}
				<div class="absolute right-2 top-1/2 -translate-y-1/2 z-10 flex">
					<Popover notClickable placement="bottom-end">
						<AlertTriangle size={16} class="text-yellow-500" />
						{#snippet text()}
							<div class="flex flex-col gap-1 text-xs">
								<span>Some search terms were ignored because they could not be parsed:</span>
								<ul class="list-disc pl-4">
									{#each queryParseErrors ?? [] as msg}
										<li>{msg}</li>
									{/each}
								</ul>
							</div>
						{/snippet}
					</Popover>
				</div>
			{/if}
		</div>
		<TimeframeSelect
			items={serviceLogsTimeframes}
			bind:value={timeframe}
			{loading}
			wrapperClasses="w-64"
			onClick={() => {
				minTs = undefined
				maxTs = undefined
				allLogs = undefined
				const ts = timeframe.computeMinMax()
				getAllLogs(ts.minTs ?? undefined, ts.maxTs ?? undefined)
			}}
		/>
		<div class="flex items-center gap-4">
			<Toggle
				size="sm"
				bind:checked={withError}
				options={{
					right: 'With errors only',
					rightTooltip: 'Only list the log files that contain at least one error line'
				}}
				on:change={() => {
					allLogs = undefined
					getAllLogs(minTs, maxTs)
				}}
			/>
			<Toggle
				size="sm"
				bind:checked={autoRefresh}
				disabled={searchTerm != ''}
				on:change={(e) => {
					if (e.detail) {
						getAllLogs(maxTs, undefined)
					} else {
						timeout && clearTimeout(timeout)
					}
				}}
				options={{
					right: 'Auto-refresh',
					rightTooltip: 'Fetch new log files every 5 seconds. Paused while searching.'
				}}
			/>
		</div>
	</div>

	{#if searchError}
		<Alert type="warning" title="Service logs search unavailable" size="xs">
			{searchError}
		</Alert>
	{/if}

	<div class="flex grow min-h-0 min-w-0 service-logs-splitpanes">
		<SplitPanesOrColumnOnMobile leftPaneSize={28} leftPaneMinSize={20} rightPaneSize={72}>
			{#snippet left_pane()}
				<div
					class="flex flex-col h-full min-h-0 overflow-y-auto mb-2 md:mb-0 md:mr-2 rounded-md border bg-surface-tertiary"
				>
					{#if allLogs == undefined}
						<div class="flex flex-col gap-2 p-3">
							{#each new Array(6) as _}
								<Skeleton layout={[[2]]} />
							{/each}
						</div>
					{:else if Object.keys(allLogs).length == 0}
						<div class="flex grow items-center justify-center p-3">
							<EmptyState
								icon={ScrollText}
								title="No log files"
								description="No service wrote a log file in this time range."
							/>
						</div>
					{:else}
						<div class="px-3 pt-3">
							<Select
								bind:value={
									() => selected,
									(v) => (v ? selectHost(v.mode, v.workerGroup, v.hostname) : (selected = v))
								}
								items={getSelectItems(allLogs, countsPerHost)}
								onClear={() => {
									selected = undefined
								}}
								size="sm"
								placeholder="Find a host"
							/>
						</div>
						{@const minTsN = minTs ? new Date(minTs).getTime() : 0}
						{@const diff = maxTs && minTs ? new Date(maxTs).getTime() - minTsN || 1 : 1}
						<div class="flex flex-col gap-4 p-3">
							{#if searchTerm === '' && minTs && maxTs}
								<!-- Same columns as a host row, so the labels sit over the bars they measure. -->
								<div
									class="flex gap-2 px-2 -mb-2 text-2xs text-secondary whitespace-nowrap"
									title="{formatTime(minTs)} – {formatTime(maxTs)}"
								>
									<span class="w-24 shrink-0"></span>
									<div class="flex grow min-w-0 justify-between gap-2">
										<span class="truncate">{formatAxisTime(minTs, maxTs)}</span>
										<span class="truncate">{formatAxisTime(maxTs, minTs)}</span>
									</div>
								</div>
							{/if}
							{#each sortedEntries(allLogsOrQueryResults(allLogs, countsPerHost)) as [mode, o1]}
								<section class="flex flex-col gap-2">
									<h2 class="text-sm font-semibold text-emphasis">{modeLabel(mode, true)}</h2>
									{#each sortedEntries(o1) as [wg, o2]}
										{@const hosts = visibleHosts(mode, wg, o2)}
										{#if hosts.length > 0}
											<div class="flex flex-col gap-0.5">
												{#if wg}
													<span class="text-2xs text-secondary px-2">{wg}</span>
												{/if}
												{#each hosts as [hn, files]}
													{@const hostKey = `${mode},${wg},${hn}`}
													{@const hostMax = Math.max(
														1,
														...files.map((f) => f.ok_lines + f.err_lines)
													)}
													<Button
														variant="subtle"
														unifiedSize="md"
														selected={isSelected(mode, wg, hn)}
														aria-current={isSelected(mode, wg, hn) ? 'true' : undefined}
														title={hn}
														wrapperClasses="w-full"
														btnClasses="w-full justify-start gap-2 px-2"
														onClick={() => selectHost(mode, wg, hn)}
													>
														<span
															class="w-24 shrink-0 truncate text-left font-mono text-2xs font-normal text-emphasis"
															>{truncateRev(hn, 14)}</span
														>
														{#if loadingLogCounts}
															<Loader2 size={16} class="ml-auto animate-spin text-secondary" />
														{:else if countsPerHost}
															<span class="ml-auto text-2xs text-secondary">
																{countsPerHost[hostKey]?.doc_count ?? 0} matches
															</span>
														{:else}
															<div
																class="relative grow h-5 border-b border-border-light"
																aria-hidden="true"
															>
																{#each files as file}
																	{@const total = file.ok_lines + file.err_lines}
																	{@const height =
																		total > 0 ? Math.max(10, (100 * total) / hostMax) : 0}
																	{@const errShare = total > 0 ? file.err_lines / total : 0}
																	<div
																		class="absolute bottom-0 w-1 -translate-x-1/2 flex flex-col"
																		style="left: {((file.ts - minTsN) / diff) *
																			100}%; height: {height}%"
																	>
																		<div class="bg-red-500" style="height: {errShare * 100}%"></div>
																		<div class="bg-border-normal grow"></div>
																	</div>
																{/each}
															</div>
														{/if}
													</Button>
												{/each}
											</div>
										{/if}
									{/each}
								</section>
							{/each}
							{#if !loadingLogCounts && sumOtherDocCount != 0}
								<span class="text-2xs text-secondary">
									{sumOtherDocCount} more matches are not attributed to any of these hosts.
								</span>
							{/if}
						</div>
					{/if}
				</div>
			{/snippet}
			{#snippet right_pane()}
				<div
					class="flex flex-col h-full min-h-0 md:ml-2 rounded-md border bg-surface-tertiary overflow-hidden"
				>
					{#if selected}
						<div
							class="flex flex-wrap items-center justify-between gap-2 px-3 py-2 border-b border-border-light"
						>
							<div class="flex flex-col gap-0.5 min-w-0">
								<div class="flex items-baseline gap-2 min-w-0">
									<h2 class="text-sm font-semibold text-emphasis">
										{modeLabel(selected.mode, false)}
									</h2>
									<span class="truncate font-mono text-2xs text-emphasis">{selected.hostname}</span>
								</div>
								{#if selected.workerGroup}
									<span class="text-2xs text-secondary">
										Worker group {selected.workerGroup}
									</span>
								{/if}
							</div>
							{#if searchTerm == ''}
								<div class="flex items-center gap-1">
									<span class="text-xs text-secondary pr-1">Last 5 log files up to</span>
									<Button
										variant="subtle"
										unifiedSize="sm"
										iconOnly
										startIcon={{ icon: ChevronLeft }}
										title="5 minutes earlier"
										disabled={!upTo}
										onClick={() => shiftUpTo(-5)}
									/>
									<div class="relative flex w-40">
										<TextInput
											size="sm"
											value={upTo ? formatTime(upTo) : ''}
											class="pr-9 tabular-nums"
											inputProps={{ readonly: true, 'aria-label': 'Logs up to' }}
										/>
										<CalendarPicker
											bind:date={upTo}
											label="Logs up to"
											placement="bottom-end"
											on:change={() => {
												upToIsLatest = false
												jumpToEnd()
											}}
										/>
									</div>
									<Button
										variant="subtle"
										unifiedSize="sm"
										iconOnly
										startIcon={{ icon: ChevronRight }}
										title="5 minutes later"
										disabled={!upTo}
										onClick={() => shiftUpTo(5)}
									/>
									<Button
										variant="default"
										unifiedSize="sm"
										selected={upToIsLatest}
										onClick={() => {
											upTo = new Date().toISOString()
											upToIsLatest = true
											jumpToEnd()
										}}
									>
										Latest
									</Button>
								</div>
							{:else if logs != undefined && !loadingLogs}
								<span class="text-2xs text-secondary">
									{(logs.hits ?? []).length} matches on this host
								</span>
							{/if}
						</div>
						{#if searchTerm == ''}
							<span class="px-3 py-1 text-2xs text-secondary border-b border-border-light">
								Logs appear about a minute after they are written, once their file is compacted.
							</span>
						{/if}
						<div class="relative grow min-h-0 flex flex-col">
							<div
								class="grow min-h-0 overflow-auto"
								id="logviewer"
								bind:this={logPane}
								onscroll={onLogPaneScroll}
							>
								{#if loadingLogs}
									<div class="flex justify-center items-center h-48 text-secondary">
										<Loader2 size={24} class="animate-spin" />
									</div>
								{:else if logs != undefined}
									<div class="flex flex-col min-w-full w-fit py-1">
										<!-- Keyed: LogSnippetViewer renders its html once at creation, so an
										 index-reused instance would keep the previous search's line. -->
										{#each logs.hits ?? [] as hit, i (`${i}:${hit.file_path}:${hit.line_no}`)}
											<LogSnippetViewer
												content={renderHit(hit)}
												highlighted={[]}
												onClick={() => seeLogContext(hit.line_no, hit.file_path, hit.host, true)}
											/>
										{/each}
										{#if (logs.hits ?? []).length === 0}
											<div class="flex items-center justify-center p-3 py-16">
												<EmptyState
													icon={SearchX}
													title="No matches on this host"
													description="Try another host, a wider time range or a broader search."
												/>
											</div>
										{/if}
										{#if (logs.hits ?? []).length === 1000}
											<span class="px-3 py-3 text-2xs text-secondary">
												Only the 1000 most recent matches are shown. Narrow the search or the time
												range to see older ones.
											</span>
										{/if}
									</div>
								{:else}
									<!-- Keyed: the window slides by one file at a time, and an index-reused
									     block would swap its content in place, so the browser could not keep
									     the line being read where it is. -->
									{#each getLogs(selected, upTo) as file (file.file_path)}
										{@const entry = logsContent[fileKey(selected.hostname, file.file_path)]}
										<div
											class="relative"
											style="min-height: {entry
												? 10
												: Math.min(file.ok_lines + file.err_lines, 30) * 16}px;"
										>
											<div
												class="sticky top-0 z-10 px-3 py-1 bg-surface-secondary border-b border-border-light text-2xs text-secondary"
											>
												{formatTime(file.ts)}
											</div>
											{#if entry == undefined}
												<div class="p-3"><Skeleton layout={[[4]]} /></div>
											{:else if entry.error}
												<div class="p-3">
													{#if entry.error?.startsWith('Not Found')}
														<Alert type="info" size="xs" title="Log file not reachable">
															Servers and workers need a shared log volume, or the EE object storage
															for logs set in the instance settings, for their log files to be
															readable from here.
														</Alert>
													{:else}
														<Alert type="error" size="xs" title="Could not load this log file">
															{entry.error}
														</Alert>
													{/if}
												</div>
											{:else if entry.content}
												<LogViewer
													noAutoScroll
													noMaxH
													isLoading={false}
													tag={undefined}
													{tagLabel}
													content={processLogWithJsonFmt(entry.content, file.json_fmt)}
												/>
											{:else}
												<p class="px-3 py-2 text-xs text-secondary">This log file is empty.</p>
											{/if}
										</div>
									{/each}
								{/if}
							</div>
							{#if showScrollToEnd && logs == undefined}
								<div
									transition:fade={{ duration: 120 }}
									class="absolute bottom-3 left-1/2 -translate-x-1/2 z-20 rounded-md bg-surface shadow-md"
								>
									<Button
										variant={hasNewLogs ? 'accent' : 'default'}
										unifiedSize="sm"
										iconOnly={!hasNewLogs}
										title="Scroll to latest logs"
										aria-label={hasNewLogs ? 'New logs, scroll to latest' : 'Scroll to latest logs'}
										startIcon={{ icon: ArrowDown }}
										onClick={() => scrollToEnd(true)}
									>
										{#if hasNewLogs}New logs{/if}
									</Button>
									{#if hasNewLogs}
										<span class="pointer-events-none absolute -top-1 -right-1 flex h-2.5 w-2.5">
											<span
												class="absolute inline-flex h-full w-full animate-ping rounded-full bg-surface-accent-primary opacity-75"
											></span>
											<span
												class="relative inline-flex h-2.5 w-2.5 rounded-full border border-surface bg-surface-accent-primary"
											></span>
										</span>
									{/if}
								</div>
							{/if}
							<!-- The button is visual only; this tells screen readers new files arrived. -->
							<span class="sr-only" aria-live="polite">
								{hasNewLogs ? 'New logs available below' : ''}
							</span>
						</div>
					{:else}
						<div class="flex grow items-center justify-center p-3">
							<EmptyState
								icon={ScrollText}
								title="Select a host"
								description="Pick a server, worker or indexer on the left to read its logs."
							/>
						</div>
					{/if}
				</div>
			{/snippet}
		</SplitPanesOrColumnOnMobile>
	</div>
</div>

<style>
	/* The two panes are separate cards; the splitter is the gap between them. */
	:global(.service-logs-splitpanes .splitpanes__splitter) {
		background-color: transparent !important;
		border: none !important;
	}
</style>
