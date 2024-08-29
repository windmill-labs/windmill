<script lang="ts">
	import { ServiceLogsService } from '$lib/gen'
	import { Pane, Splitpanes } from 'svelte-splitpanes'

	import ManuelDatePicker from './runs/ManuelDatePicker.svelte'
	import CalendarPicker from './common/calendarPicker/CalendarPicker.svelte'
	import LogViewer from './LogViewer.svelte'
	import Toggle from './Toggle.svelte'
	import { sendUserToast } from '$lib/toast'
	import { onDestroy } from 'svelte'
	import { Loader2 } from 'lucide-svelte'

	let minTs: undefined | string = undefined
	let maxTs: undefined | string = undefined

	let minTsManual: undefined | string = undefined
	let maxTsManual: undefined | string = undefined

	let max_lines: undefined | number = undefined

	// let lastSeen: undefined | string = undefined

	let withError = false
	let autoRefresh = true
	let loading = false

	type LogFile = {
		ts: number
		file_path: string
		ok_lines: number
		err_lines: number
	}

	type ByHostname = Record<string, LogFile[]>
	type ByWorkerGroup = Record<string, ByHostname>
	type ByMode = Record<string, ByWorkerGroup>

	let timeout: NodeJS.Timeout | undefined = undefined

	let allLogs: ByMode | undefined = undefined

	function getAllLogs(queryMinTs: string | undefined, queryMaxTs: string | undefined) {
		timeout && clearTimeout(timeout)
		loading = true
		let nallLogs = structuredClone(allLogs) ?? {}
		ServiceLogsService.listLogFiles({ withError, before: queryMaxTs, after: queryMinTs })
			.then((res) => {
				let minTsN = queryMinTs ? new Date(queryMinTs).getTime() : undefined
				let maxTsN = queryMaxTs ? new Date(queryMaxTs).getTime() : undefined
				res.forEach((log) => {
					let ts = new Date(log.log_ts).getTime()
					if (minTsN == undefined || ts < minTsN) {
						minTsN = ts
					}
					if (maxTsN == undefined || ts > maxTsN) {
						maxTsN = ts
					}
					if (!nallLogs[log.mode]) {
						nallLogs[log.mode] = {}
					}
					const wg = log.worker_group ?? ''
					if (!nallLogs[log.mode][wg]) {
						nallLogs[log.mode][wg] = {}
					}
					const hn = log.hostname ?? ''
					if (!nallLogs[log.mode][wg][hn]) {
						nallLogs[log.mode][wg][hn] = []
					}
					nallLogs[log.mode][wg][hn].push({
						ts: ts,
						file_path: log.file_path,
						ok_lines: log.ok_lines ?? 1,
						err_lines: log.err_lines ?? 0
					})
					if (
						log.ok_lines != undefined &&
						log.err_lines != undefined &&
						(max_lines == undefined || log.ok_lines + log.err_lines > max_lines)
					) {
						max_lines = log.ok_lines + log.err_lines
					}
				})
				console.log(minTs, minTsN, maxTsN)
				if (minTs == undefined) {
					minTs = minTsN ? new Date(minTsN).toISOString() : undefined
				}
				maxTs = maxTsN ? new Date(maxTsN).toISOString() : undefined
				allLogs = nallLogs
				loading = false
				if (autoRefresh && !queryMaxTs) {
					timeout && clearTimeout(timeout)
					let localMaxTs = maxTs
					timeout = setTimeout(() => {
						getAllLogs(localMaxTs, undefined)
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

	let selected: [string, string, string] | undefined = undefined

	let logsContent: Record<string, { content?: string; error?: string }> = {}
	export async function getLogFile(path: string) {
		if (logsContent[path]) {
			return
		}
		try {
			const res = await ServiceLogsService.getLogFile({ path: path })
			logsContent[path] = { content: res }
		} catch (e) {
			logsContent[path] = { error: e.message }
		}
	}

	getAllLogs(undefined, undefined)

	let upTo: undefined | string = undefined

	function getLogs(selected: [string, string, string], upTo: string | undefined) {
		if (!selected) {
			return []
		}
		let logs = allLogs?.[selected[0]]?.[selected[1]]?.[selected[2]]
		if (!logs) {
			return []
		}
		if (upTo) {
			let upToN = new Date(upTo).getTime()
			logs = logs
				.filter((x) => x.ts <= upToN)
				.slice(undefined, 5)
				.reverse()
			getFiles(logs.map((x) => x.file_path))
		}
		return logs
	}

	async function getFiles(logs: string[]) {
		await Promise.all(logs.map((x) => getLogFile(x)))
		scrollToBottom()
	}

	function getLatestUpTo(selected: [string, string, string]): any {
		if (!selected) {
			return undefined
		}
		let logs = allLogs?.[selected[0]]?.[selected[1]]?.[selected[2]]
		return logs?.[0]?.ts
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
</script>

{minTs}
{maxTs}
<div class="w-full h-[70vh]" on:scroll|preventDefault>
	<Splitpanes>
		<Pane size={40} minSize={20}>
			<div class="p-1">
				<div
					class="flex flex-col lg:flex-row gap-y-1 justify-between w-full relative pb-4 gap-x-0.5"
					id="service-logs-date-pickers"
				>
					<div class="flex relative">
						<input
							type="text"
							value={minTsManual
								? new Date(minTsManual).toLocaleTimeString([], {
										day: '2-digit',
										month: '2-digit',
										hour: '2-digit',
										minute: '2-digit'
								  })
								: 'min datetime'}
							disabled
						/>
						<CalendarPicker label="min datetime" date={minTs} />
					</div>
					<ManuelDatePicker
						bind:minTs={minTsManual}
						bind:maxTs={maxTsManual}
						{loading}
						on:loadJobs={() => {
							minTs = undefined
							maxTs = undefined
							allLogs = undefined
							getAllLogs(minTsManual, maxTsManual)
							console.log('load jobs')
						}}
						serviceLogsChoices
						loadText="Last 1000 logfiles"
					/>
					<div class="flex relative">
						<input
							type="text"
							value={maxTsManual
								? new Date(maxTsManual).toLocaleTimeString([], {
										day: '2-digit',
										month: '2-digit',
										hour: '2-digit',
										minute: '2-digit'
								  })
								: 'max datetime'}
							disabled
						/>
						<CalendarPicker label="max datetime" date={maxTs} />
					</div>
				</div>
				<div class="flex w-full flex-row-reverse pb-4 -mt-2 gap-2"
					><Toggle size="xs" bind:checked={withError} options={{ right: 'error only' }} />
					<Toggle
						size="xs"
						bind:checked={autoRefresh}
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
					{#each Object.entries(allLogs) as [mode, o1]}
						<div class="w-full pb-6">
							<h2 class="pb-2">{mode}s</h2>
							{#each Object.entries(o1) as [wg, o2]}
								<div class="w-full">
									{#if wg && wg != ''}
										<h3 class="pb-2">{wg}</h3>
									{/if}
									<div class="divide-y flex flex-col">
										{#each Object.entries(o2) as [hn, files]}
											<!-- svelte-ignore a11y-click-events-have-key-events -->
											<!-- svelte-ignore a11y-no-static-element-interactions -->
											<div
												class="w-full flex items-baseline px-1 cursor-pointer {selected &&
												selected[0] == mode &&
												selected[1] == wg &&
												selected[2] == hn
													? 'bg-surface-secondary'
													: ''}"
												on:click={() => {
													selected = [mode, wg, hn]
													upTo = getLatestUpTo(selected)
													scrollToBottom()
												}}
											>
												<div style="width: 90px;">{hn}</div>
												<div class="relative grow h-8 mr-2">
													{#each files as file}
														{@const okHeight = 100.0 * ((file.ok_lines * 1.0) / (max_lines ?? 1))}
														{@const errHeight = 100.0 * ((file.err_lines * 1.0) / (max_lines ?? 1))}
														<div
															class=" w-2 bg-red-400 absolute"
															style="left: {((file.ts - minTsN) / diff) *
																100}%; height: {errHeight}%; bottom: {okHeight}%;"
														/>
														<div
															class=" w-2 bg-black absolute bottom-0"
															style="left: {((file.ts - minTsN) / diff) *
																100}%; height: {okHeight}%"
														/>
													{/each}
												</div>
											</div>
										{/each}
									</div>
								</div>
							{/each}
						</div>
					{/each}
				{/if}
			</div>
		</Pane>
		<Pane size={60} minSize={20}
			><div class="relative h-full flex flex-col gap-1"
				>{#if selected}
					<div class="grow overflow-auto" id="logviewer">
						{#each getLogs(selected, upTo) as file}
							<div
								style="min-height: {logsContent[file.file_path]
									? 10
									: (file.ok_lines + file.err_lines) / 20}px;"
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
									<div>Loading...</div>
								{:else if logsContent[file.file_path]}
									{#if logsContent[file.file_path].error}
										<div>{logsContent[file.file_path].error}</div>
									{:else if logsContent[file.file_path].content}
										<!-- svelte-ignore a11y-click-events-have-key-events -->
										<!-- svelte-ignore a11y-no-static-element-interactions -->
										<div on:click|preventDefault
											><LogViewer
												noAutoScroll
												noMaxH
												isLoading={false}
												tag={undefined}
												content={logsContent[file.file_path].content}
											/></div
										>
									{:else}
										<div>No logs</div>
									{/if}
								{/if}
							</div>
						{/each}
					</div>
					<div class="flex w-full items-center">
						<div class="flex grow text-xs justify-between px-2">
							{#if upTo}
								<button
									on:click={() => {
										if (upTo) {
											upTo = new Date(new Date(upTo).getTime() - 5 * 60 * 1000).toISOString()
										}
									}}>{'<'} prev 5 minutes</button
								>
							{:else}
								<div />
							{/if}

							<div class="flex gap-1 relative items-center"
								>Up to <div class="flex gap-1 relative">
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
									on:click={() => {
										if (upTo) {
											upTo = new Date(new Date(upTo).getTime() + 5 * 60 * 1000).toISOString()
										}
									}}>next 5 minutes {'>'}</button
								>
							{:else}
								<div />
							{/if}
						</div>
						<div>
							<button
								on:click={() => {
									upTo = new Date().toISOString()
								}}>now</button
							>
						</div>
					</div>
				{:else}
					<div class="flex justify-center items-center">Select a host to see its logs</div>
				{/if}</div
			></Pane
		>
	</Splitpanes>
</div>
