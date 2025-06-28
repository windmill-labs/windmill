<script lang="ts">
	import { debounce, displayDate, msToSec } from '$lib/utils'
	import { onDestroy } from 'svelte'
	import { getDbClockNow } from '$lib/forLater'
	import { Loader2 } from 'lucide-svelte'
	import TimelineBar from './TimelineBar.svelte'
	import type { Writable } from 'svelte/store'
	import WaitTimeWarning from './common/waitTimeWarning/WaitTimeWarning.svelte'

	export let selfWaitTime: number | undefined = undefined
	export let aggregateWaitTime: number | undefined = undefined
	export let flowModules: string[]
	export let durationStatuses: Writable<
		Record<
			string,
			{
				byJob: Record<string, { created_at?: number; started_at?: number; duration_ms?: number }>
				iteration_from?: number
				iteration_total?: number
			}
		>
	>
	export let flowDone = false

	let min: undefined | number = undefined
	let max: undefined | number = undefined
	let total: number | undefined = undefined

	let items:
		| Record<
				string,
				Array<{ created_at?: number; started_at?: number; duration_ms?: number; id: string }>
		  >
		| undefined = undefined

	let { debounced, clearDebounce } = debounce(() => computeItems($durationStatuses), 30)
	$: flowDone != undefined && $durationStatuses && debounced()

	export function reset() {
		min = undefined
		max = undefined
		items = computeItems($durationStatuses)
	}

	function computeItems(
		durationStatuses: Record<
			string,
			{
				byJob: Record<string, { created_at?: number; started_at?: number; duration_ms?: number }>
			}
		>
	): any {
		let nmin: undefined | number = undefined
		let nmax: undefined | number = undefined

		let isStillRunning = false

		let cnt = 0
		let nitems = {}
		Object.entries(durationStatuses).forEach(([k, o]) => {
			Object.values(o.byJob).forEach((v) => {
				cnt++
				if (v.started_at) {
					if (!nmin) {
						nmin = v.started_at
					} else {
						nmin = Math.min(nmin, v.started_at)
					}
				}
				if (!flowDone && v.duration_ms == undefined) {
					isStillRunning = true
				}

				if (!isStillRunning) {
					if (v.started_at && v.duration_ms != undefined) {
						let lmax = v.started_at + v.duration_ms
						if (!nmax) {
							nmax = lmax
						} else {
							nmax = Math.max(nmax, lmax)
						}
					}
				}
			})
			let arr = Object.entries(o.byJob).map(([k, v]) => ({ ...v, id: k }))
			arr.sort((x, y) => {
				if (!x.started_at) {
					return -1
				} else if (!y.started_at) {
					return 1
				} else {
					return x.started_at - y.started_at
				}
			})

			nitems[k] = arr
		})
		items = nitems
		min = nmin
		max = isStillRunning || (cnt < flowModules.length && !flowDone) ? undefined : nmax
		if (max && min) {
			total = max - min
			total = Math.max(total, 2000)
		}
	}

	let now = getDbClockNow().getTime()

	let interval = setInterval((x) => {
		if (!max) {
			now = getDbClockNow().getTime()
		}
		if (min && (!max || total == undefined)) {
			total = max ? max - min : Math.max(now - min, 2000)
		}
	}, 30)

	onDestroy(() => {
		interval && clearInterval(interval)
		clearDebounce()
	})
</script>

{#if items}
	<div class="divide-y border-b">
		<div class="px-2 py-2 grid grid-cols-12 w-full"
			><div></div>
			<div class="col-span-11 pt-1 px-2 flex text-2xs text-secondary justify-between"
				><div>{min ? displayDate(new Date(min), true) : ''}</div>{#if max && min}<div
						class="hidden lg:block">{msToSec(max - min)}s</div
					>
				{/if}<div class="flex gap-1 items-center font-mono"
					>{max ? displayDate(new Date(max), true) : ''}{#if !max && min}{#if now}
							{msToSec(now - min, 3)}s
						{/if}<Loader2 size={14} class="animate-spin" />{/if}</div
				></div
			>
		</div>
		<div class="flex flex-row-reverse items-center text-sm text-secondary p-2">
			<div class="flex gap-4 items-center text-2xs">
				<div class="flex gap-2 items-center">
					<div>Waiting for executor/Suspend</div>
					<div class="h-4 w-4 bg-gray-500"></div>
				</div>

				<div class="flex gap-2 items-center">
					<div>Execution</div>
					<div class="h-4 w-4 bg-blue-500/90"></div>
				</div>
			</div>
		</div>
		{#if selfWaitTime}
			<div class="px-2 py-2 grid grid-cols-6 w-full">
				root:
				<WaitTimeWarning
					self_wait_time_ms={selfWaitTime}
					aggregate_wait_time_ms={aggregateWaitTime}
					variant="badge-self-wait"
				/>
			</div>
		{/if}
		{#each Object.values(flowModules) as k (k)}
			<div class="overflow-auto max-h-60 shadow-inner dark:shadow-gray-700 relative">
				{#if ($durationStatuses?.[k]?.iteration_from ?? 0) > 0}
					<div class="w-full flex flex-row-reverse sticky top-0">
						<button
							class="!text-secondary underline mr-2 text-2xs text-right whitespace-nowrap"
							on:click={() => {
								let r = $durationStatuses[k]
								if (r.iteration_from) {
									r.iteration_from -= 20
									$durationStatuses = $durationStatuses
								}
							}}
							>Viewing iterations {$durationStatuses[k].iteration_from} to {$durationStatuses[k]
								.iteration_total}. Load more
						</button>
					</div>
				{/if}

				<div class="px-2 py-2 grid grid-cols-6 w-full">
					<div class="truncate">{k.startsWith('subflow:') ? k.substring(8) : k}</div>
					<div class="col-span-5 flex min-h-6">
						{#if min && total}
							<div class="flex flex-col gap-2 w-full p-2 ml-4">
								{#each items?.[k] ?? [] as b}
									{@const waitingLen = b?.created_at
										? b.started_at
											? b.started_at - b?.created_at
											: b.duration_ms
												? 0
												: now - b?.created_at
										: 0}
									<div class="flex w-full">
										<TimelineBar
											position="left"
											id={b?.id}
											{total}
											{min}
											gray
											started_at={b.created_at}
											len={waitingLen < 100 ? 0 : waitingLen - 100}
											running={b?.started_at == undefined}
										/>
										{#if b.started_at}
											<TimelineBar
												position={waitingLen < 100 ? 'center' : 'right'}
												id={b?.id}
												{total}
												{min}
												concat
												started_at={b.started_at}
												len={b.started_at ? (b?.duration_ms ?? now - b?.started_at) : 0}
												running={b?.duration_ms == undefined}
											/>
										{/if}
									</div>
								{/each}
							</div>
						{/if}</div
					></div
				>
			</div>
		{/each}
	</div>
{:else}
	<Loader2 class="animate-spin" />
{/if}
