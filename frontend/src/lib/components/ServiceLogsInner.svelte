<script lang="ts">
	import { ServiceLogsService } from '$lib/gen'

	let minTs: undefined | number = undefined
	let maxTs: undefined | number = undefined

	type LogFile = {
		ts: number
		file_path: string
		size: number
	}

	type ByHostname = Record<string, LogFile[]>
	type ByWorkerGroup = Record<string, ByHostname>
	type ByMode = Record<string, ByWorkerGroup>

	let logs: ByMode = {}
	function getLogs() {
		ServiceLogsService.listLogFiles({}).then((res) => {
			res.forEach((log) => {
				let ts = new Date(log.log_ts).getTime()
				if (minTs == undefined || ts < minTs) {
					minTs = ts
				}
				if (maxTs == undefined || ts > maxTs) {
					maxTs = ts
				}
				if (!logs[log.mode]) {
					logs[log.mode] = {}
				}
				const wg = log.worker_group ?? ''
				if (!logs[log.mode][wg]) {
					logs[log.mode][wg] = {}
				}
				const hn = log.hostname ?? ''
				if (!logs[log.mode][wg][hn]) {
					logs[log.mode][wg][hn] = []
				}
				logs[log.mode][wg][hn].push({
					ts: ts,
					file_path: log.file_path,
					size: 1
				})
			})
		})
	}

	let selected: [string, string, string] | undefined = undefined

	getLogs()
</script>

{#if logs}
	<div class="w-full">
		<div class="flex gap-8 text-xs pb-2">
			<div>minTS: {minTs}</div>
			<div>maxTS: {maxTs}</div>
		</div>
		{#if minTs && maxTs}
			{#each Object.entries(logs) as [mode, o1]}
				<div class="w-full">
					<h2 class="pb-2">{mode}</h2>
					{#each Object.entries(o1) as [wg, o2]}
						<div class="w-full">
							<h3>{wg}</h3>
							<div class="border">
								{#each Object.entries(o2) as [hn, files]}
									<div
										class="w-full flex items-baseline px-1 {selected &&
										selected[0] == mode &&
										selected[1] == wg &&
										selected[2] == hn
											? 'bg-slate-100'
											: ''}"
										on:click={() => {
											selected = [mode, wg, hn]
										}}
									>
										<h4 style="width: 90px;">{hn}</h4>
										<div class="relative grow h-4">
											{#each files as file}
												<div
													class=" h-2 w-2 bg-black absolute bottom-0"
													style="left: {((file.ts - minTs) / (maxTs - minTs)) * 100}%;"
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
{/if}
