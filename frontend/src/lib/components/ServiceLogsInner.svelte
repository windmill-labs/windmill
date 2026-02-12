<script lang="ts">
	import { createBubbler, preventDefault } from 'svelte/legacy'

	const bubble = createBubbler()
	import { IndexSearchService, ServiceLogsService } from '$lib/gen'

	import TimeframeSelect, {
		serviceLogsTimeframes,
		useUrlSyncedTimeframe
	} from './runs/TimeframeSelect.svelte'
	import CalendarPicker from './common/calendarPicker/CalendarPicker.svelte'
	import LogViewer from './LogViewer.svelte'
	import Toggle from './Toggle.svelte'
	import { sendUserToast } from '$lib/toast'
	import { onDestroy, tick } from 'svelte'
	import { Loader2 } from 'lucide-svelte'
	import { copyToClipboard, scroll_into_view_if_needed_polyfill, truncateRev } from '$lib/utils'
	import LogSnippetViewer from './LogSnippetViewer.svelte'
	import { Button, Drawer, DrawerContent } from './common'
	import ClipboardCopy from 'lucide-svelte/icons/clipboard-copy'
	import { AnsiUp } from 'ansi_up'
	import SplitPanesOrColumnOnMobile from './splitPanes/SplitPanesOrColumnOnMobile.svelte'
	import Select from './select/Select.svelte'
	import { goto } from '$lib/navigation'
	import { page } from '$app/stores'
	import { watch } from 'runed'

	interface Props {
		searchTerm: string
		queryParseErrors?: string[]
		tagLabel?: string
	}

	let { searchTerm, queryParseErrors = $bindable(), tagLabel }: Props = $props()

	let minTs: undefined | string = $state(undefined)
	let maxTs: undefined | string = $state(undefined)

	let max_lines: undefined | number = $state(undefined)

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
					allLogs[log.mode][wg][hn].push({
						ts: ts,
						file_path: log.file_path,
						ok_lines: log.ok_lines ?? 1,
						err_lines: log.err_lines ?? 0,
						json_fmt: log.json_fmt
					})
					if (
						log.ok_lines != undefined &&
						log.err_lines != undefined &&
						(max_lines == undefined || log.ok_lines + log.err_lines > max_lines)
					) {
						max_lines = log.ok_lines + log.err_lines
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
						let maxTsPlus1 = maxTs ? new Date(new Date(maxTs).getTime() + 1000) : undefined
						getAllLogs(maxTsPlus1?.toISOString(), undefined)
					}, 5000)
				}
			})
			.catch((e) => {
				sendUserToast('Failed to load service logs: ' + e.body, true)
				console.error(e)
				loading = false
				autoRefresh = false
			})
	}

	type Selected = { mode: string; workerGroup: string; hostname: string }
	let initialSelected =
		$page.url.searchParams.get('mode') &&
		$page.url.searchParams.get('workerGroup') &&
		$page.url.searchParams.get('hostname')
			? {
					mode: $page.url.searchParams.get('mode')!,
					workerGroup: $page.url.searchParams.get('workerGroup')!,
					hostname: $page.url.searchParams.get('hostname')!
				}
			: undefined
	let selected: Selected | undefined = $state(initialSelected)

	let logsContent: Record<string, { content?: string; error?: string }> = $state({})
	export async function getLogFile(hostname: string, path: string) {
		if (logsContent[path]) {
			return
		}
		try {
			const res = await ServiceLogsService.getLogFile({ path: `${hostname}/${path}` })
			logsContent[path] = { content: res }
		} catch (e) {
			logsContent[path] = { error: `${e.message}: ${e.body}` }
		}
	}

	getAllLogs(undefined, undefined)

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
		await Promise.all(logs.map((x) => getLogFile(hostname, x)))
		scrollToBottom()
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

	function scrollToBottom() {
		const el = document.querySelector('#logviewer')
		if (el) {
			el.scrollTop = el.scrollHeight
		}
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
				if (line.startsWith('{') && line.endsWith('}')) {
					let obj = JSON.parse(line)
					if (typeof obj == 'object') {
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
								key +
								'=' +
								(typeof obj[key] == 'object' ? JSON.stringify(obj[key]) : obj[key]) +
								' '
						})
						res += nl + '\n'
					}
				}
			})

			return res
		} catch (e) {
			return log
		}
	}

	let logs: any = $state()

	let debounceTimeout: number | undefined = undefined
	const debouncePeriod: number = 400
	let loadingLogs = $state(false)
	let loadingLogCounts = $state(false)

	let countsPerHost: any = $state()
	let sumOtherDocCount: number = $state(0)

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
		goto(`?${params.toString()}`)
		if (searchTerm.trim() === '') {
			debounceTimeout && clearTimeout(debounceTimeout)
			logs = undefined
			countsPerHost = undefined
			sumOtherDocCount = 0
			loadingLogs = false
			loadingLogCounts = false
			return
		}
		timeout && clearTimeout(timeout)

		loadingLogCounts = true
		loadingLogs = true
		debounceTimeout && clearTimeout(debounceTimeout)
		debounceTimeout = setTimeout(async () => {
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
				loadingLogCounts = false
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

			loadingLogs = false
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

		content = processLogWithJsonFmt(ansi_up.ansi_to_html(res), jsonFmt)
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
	<DrawerContent title="See context" on:close={logDrawer.closeDrawer}>
		{#snippet actions()}
			<Button
				on:click={() => copyToClipboard(content)}
				color="light"
				size="xs"
				startIcon={{
					icon: ClipboardCopy
				}}
			>
				Copy to clipboard
			</Button>
		{/snippet}
		<div class="w-fit">
			<pre
				class="bg-surface-secondary text-secondary text-xs w-full p-2 whitespace-pre border rounded-md"
				>...<br />{#each content.split('\n') as line, index}<div
						id={`log-line-${index}`}
						class={index === hitLineNumber ? 'bg-yellow-200 bg-opacity-20' : ''}>{@html line}</div
					>{/each}...</pre
			>
		</div>
	</DrawerContent>
</Drawer>

<SplitPanesOrColumnOnMobile>
	{#snippet left_pane()}
		<div class="p-1">
			<div
				class="flex flex-col lg:flex-row gap-y-1 justify-between w-full relative pb-4 gap-x-0.5"
				id="service-logs-date-pickers"
			>
				<TimeframeSelect
					items={serviceLogsTimeframes}
					bind:value={timeframe}
					{loading}
					wrapperClasses="w-full"
					onClick={() => {
						minTs = undefined
						maxTs = undefined
						allLogs = undefined
						const ts = timeframe.computeMinMax()
						getAllLogs(ts.minTs ?? undefined, ts.maxTs ?? undefined)
					}}
				/>
			</div>
			<div class="flex w-full flex-row-reverse pb-4 -mt-2 gap-2"
				><Toggle
					size="xs"
					bind:checked={withError}
					options={{ right: 'errors > 0' }}
					on:change={() => {
						allLogs = undefined
						getAllLogs(minTs, maxTs)
					}}
				/>
				<Toggle
					size="xs"
					bind:checked={autoRefresh}
					disabled={searchTerm != ''}
					on:change={(e) => {
						if (e.detail) {
							getAllLogs(maxTs, undefined)
						} else {
							timeout && clearTimeout(timeout)
						}
					}}
					options={{ right: 'auto-refresh' }}
				/></div
			>
			{#if allLogs == undefined}
				<div class="text-center pb-2"><Loader2 class="animate-spin" /></div>
			{:else if Object.keys(allLogs).length == 0}
				<div class="flex justify-center items-center h-full">No logs</div>
			{:else if minTs && maxTs}
				{@const minTsN = new Date(minTs).getTime()}
				{@const maxTsN = new Date(maxTs).getTime()}
				{@const diff = maxTsN - minTsN}
				{#if searchTerm === ''}
					<div class="flex w-full text-2xs text-primary pb-6">
						<div style="width: 60px;"></div>

						<div class="flex justify-between w-full"
							><div
								>{new Date(minTs).toLocaleTimeString([], {
									day: '2-digit',
									month: '2-digit',
									hour: '2-digit',
									minute: '2-digit'
								})}</div
							><div
								>{new Date(maxTs).toLocaleTimeString([], {
									day: '2-digit',
									month: '2-digit',
									hour: '2-digit',
									minute: '2-digit'
								})}</div
							></div
						>
					</div>
				{/if}
				<div class="mr-0.5 mb-2">
					<Select
						bind:value={selected}
						items={getSelectItems(allLogs, countsPerHost)}
						onClear={() => {
							selected = undefined
						}}
						placeholder="Select a service"
					/>
				</div>
				{#each Object.entries(allLogsOrQueryResults(allLogs, countsPerHost)) as [mode, o1]}
					<div class="w-full pb-8">
						<h2 class="pb-2 text-2xl">{mode}s</h2>
						{#each Object.entries(o1) as [wg, o2]}
							<div class="w-full px-1">
								{#if wg && wg != ''}
									<h4 class="pt-4">{wg}</h4>
								{/if}
								<div class="divide-y flex flex-col">
									{#each Object.entries(o2).filter(([hn, files]) => {
										if (selected && selected.mode === mode && selected.workerGroup === wg && selected.hostname === hn) {
											return true
										}
										const hostKey = `${mode},${wg},${hn}`
										if (countsPerHost && (countsPerHost[hostKey] == undefined || countsPerHost[hostKey].doc_count === 0)) {
											return false
										}
										return true
									}) as [hn, files]}
										{@const hostKey = `${mode},${wg},${hn}`}
										<!-- svelte-ignore a11y_click_events_have_key_events -->
										<!-- svelte-ignore a11y_no_static_element_interactions -->
										<div
											class="w-full flex items-baseline justify-between rounded px-1 hover:bg-surface-hover cursor-pointer {selected &&
											selected.mode == mode &&
											selected.workerGroup == wg &&
											selected.hostname == hn
												? 'bg-surface-secondary'
												: ''}"
											onclick={() => {
												selected = { mode, workerGroup: wg, hostname: hn }
												upToIsLatest = true
												upTo = getLatestUpTo(selected)
												scrollToBottom()
											}}
										>
											<div
												class="text-sm pt-2 pl-0.5 whitespace-nowrap"
												title={hn}
												style="width: 90px;"
												>{truncateRev(hn, countsPerHost || loadingLogs ? 40 : 8)}</div
											>
											{#if loadingLogCounts}
												<Loader2 size={15} class="animate-spin" />
											{:else if countsPerHost}
												<div class="text-primary text-xs">
													{countsPerHost[hostKey]?.doc_count ?? 0} matches
												</div>
											{:else}
												<div class="relative grow h-8 mr-2">
													{#each files as file}
														{@const okHeight = 100.0 * ((file.ok_lines * 1.0) / (max_lines ?? 1))}
														{@const errHeight = 100.0 * ((file.err_lines * 1.0) / (max_lines ?? 1))}
														<div
															class=" w-2 bg-red-400 absolute"
															style="left: {((file.ts - minTsN) / diff) *
																100}%; height: {errHeight}%; bottom: {okHeight}%;"
														></div>
														<div
															class="w-2 bg-surface-secondary-inverse absolute bottom-0"
															style="left: {((file.ts - minTsN) / diff) *
																100}%; height: {okHeight}%"
														></div>
													{/each}
												</div>
											{/if}
										</div>
									{/each}
								</div>
							</div>
						{/each}
					</div>
				{/each}
				{#if !loadingLogCounts && sumOtherDocCount != 0}
					<div class="text-primary italic text-sm">
						Note: {sumOtherDocCount} additional matches weren't grouped into any of the above hosts.
					</div>
				{/if}
			{/if}
		</div>
	{/snippet}
	{#snippet right_pane()}
		<div class="relative h-full flex flex-col gap-1 pb-2">
			{#if selected}
				{#if !loadingLogs && logs == undefined}
					<div class="w-full bg-surface-primary-inverse text-primary text-xs text-center">
						1 min delay: logs are compacted before being available
					</div>
				{/if}
				<div class="grow overflow-auto" id="logviewer">
					{#if loadingLogs}
						<div class="flex w-full justify-center items-center h-48">
							<div class="text-primary text-center">
								<Loader2 size={34} class="animate-spin" />
							</div>
						</div>
					{:else if logs != undefined}
						<div class="flex flex-col min-w-full w-fit">
							{#each logs.hits as { snippet_fragment, snippet_highlighted, document }}
								<LogSnippetViewer
									content={snippet_fragment || document.logs[0]}
									highlighted={snippet_highlighted}
									on:click={() => {
										let logLineNumber = document.line_number[0]
										let logFile = document.file_name[0]
										let host = document.host[0]
										let jsonFmt = document.json_fmt[0]
										seeLogContext(logLineNumber, logFile, host, jsonFmt)
									}}
								/>
							{/each}
							{#if logs.hits.length === 0}
								<div class="text-center py-20 text-bold text-xl text-primary"> No logs </div>
							{/if}
							{#if logs.hits.length === 1000}
								<div class="pl-6 py-6 text-sm text-secondary">
									Older matches were truncated from this search, try refining your filters to get
									more precise results.
								</div>
							{/if}
							<div class="py-20"></div>
						</div>
					{:else}
						{#each getLogs(selected, upTo) as file}
							<div
								style="min-height: {logsContent[file.file_path]
									? 10
									: Math.min(file.ok_lines + file.err_lines, 30) * 16}px;"
							>
								<div class="bg-surface-primary-inverse text-sm font-semibold px-1"
									>{new Date(file.ts).toLocaleTimeString([], {
										day: '2-digit',
										month: '2-digit',
										hour: '2-digit',
										minute: '2-digit'
									})}</div
								>
								{#if logsContent[file.file_path] == undefined}
									<div
										class="animate-skeleton dark:bg-frost-900/50 [animation-delay:1000ms] h-full w-full"
									></div>
								{:else if logsContent[file.file_path]}
									{#if logsContent[file.file_path].error}
										{#if logsContent[file.file_path].error?.startsWith('Not Found')}
											<div class="text-xs pb-4 pt-2 text-secondary"
												>Log file is missing. Log files require a shared log volume to be mounted
												across servers and workers or to use the EE S3/object storage integration
												for logs. To avoid mounting a shared volume, set the EE object store logs in
												the instance settings</div
											>
										{:else}
											<div class="text-xs text-red-400 pb-4"
												>{logsContent[file.file_path].error}</div
											>
										{/if}
									{:else if logsContent[file.file_path].content}
										<!-- svelte-ignore a11y_click_events_have_key_events -->
										<!-- svelte-ignore a11y_no_static_element_interactions -->
										<div onclick={preventDefault(bubble('click'))} class="pr-2"
											><LogViewer
												noAutoScroll
												noMaxH
												isLoading={false}
												tag={undefined}
												{tagLabel}
												content={processLogWithJsonFmt(
													logsContent[file.file_path].content,
													file.json_fmt
												)}
											/></div
										>
									{:else}
										<div>No logs</div>
									{/if}
								{/if}
							</div>
						{/each}
					{/if}
				</div>
				{#if searchTerm == ''}
					<div class="flex w-full items-center gap-4">
						<div class="text-primary px-1 text-2xs">Last 5 log files up to:</div>
						<div class="flex grow text-xs justify-center px-2 items-center gap-2">
							{#if upTo}
								<button
									onclick={() => {
										if (upTo) {
											upToIsLatest = false
											upTo = new Date(new Date(upTo).getTime() - 5 * 60 * 1000).toISOString()
										}
									}}>{'<'} 5m</button
								>
							{:else}
								<div></div>
							{/if}

							<div class="flex gap-1 relative items-center"
								><div class="flex gap-1 relative">
									<input
										type="text"
										value={upTo
											? new Date(upTo).toLocaleTimeString([], {
													day: '2-digit',
													month: '2-digit',
													hour: '2-digit',
													minute: '2-digit'
												})
											: ''}
										disabled
									/><CalendarPicker bind:date={upTo} label="Logs up to" /></div
								></div
							>
							{#if upTo}
								<button
									onclick={() => {
										if (upTo) {
											upToIsLatest = false
											upTo = new Date(new Date(upTo).getTime() + 5 * 60 * 1000).toISOString()
										}
									}}>5m {'>'}</button
								>
							{:else}
								<div></div>
							{/if}
						</div>
						<div>
							<button
								class="text-xs"
								onclick={() => {
									upTo = new Date().toISOString()
									upToIsLatest = true
								}}>now</button
							>
						</div>
					</div>
				{/if}
			{:else}
				<div class="flex justify-center items-center pt-8">Select a host to see its logs</div>
			{/if}</div
		>
	{/snippet}
</SplitPanesOrColumnOnMobile>
