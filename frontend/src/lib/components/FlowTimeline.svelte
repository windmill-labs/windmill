<script lang="ts">
	import { debounce, displayDate, msToSec } from '$lib/utils'
	import { onDestroy } from 'svelte'
	import { getDbClockNow } from '$lib/forLater'
	import { Loader2 } from 'lucide-svelte'
	import TimelineBar from './TimelineBar.svelte'

	export let flowModules: string[]
	export let durationStatuses: Record<
		string,
		Record<string, { started_at?: number; duration_ms?: number }>
	>

	let min: undefined | number = undefined
	let max: undefined | number = undefined
	let total: number | undefined = undefined

	let items:
		| Record<string, Array<{ started_at?: number; duration_ms?: number; id: string }>>
		| undefined = undefined

	let debounced = debounce(() => computeItems(durationStatuses), 30)
	$: durationStatuses && debounced()

	export function reset() {
		min = undefined
		max = undefined
		items = computeItems(durationStatuses)
	}

	function computeItems(
		durationStatuses: Record<string, Record<string, { started_at?: number; duration_ms?: number }>>
	): any {
		let nmin: undefined | number = undefined
		let nmax: undefined | number = undefined

		let isStillRunning = false

		let cnt = 0
		let nitems = {}
		Object.entries(durationStatuses).forEach(([k, o]) => {
			Object.values(o).forEach((v) => {
				cnt++
				if (v.started_at) {
					if (!nmin) {
						nmin = v.started_at
					} else {
						nmin = Math.min(nmin, v.started_at)
					}
				}
				if (v.duration_ms == undefined) {
					isStillRunning = true
				}
				if (!isStillRunning) {
					if (v.started_at && v.duration_ms) {
						let lmax = v.started_at + v.duration_ms
						if (!nmax) {
							nmax = lmax
						} else {
							nmax = Math.max(nmax, lmax)
						}
					}
				}
			})
			let arr = Object.entries(o).map(([k, v]) => ({ ...v, id: k }))
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
		max = isStillRunning || cnt < flowModules.length ? undefined : nmax
		if (max && min) {
			total = max - min
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
	})
</script>

{#if items}
	<div class="divide-y">
		<div class="px-2 py-2 grid grid-cols-12 w-full"
			><div />
			<div class="col-span-11 pt-1 px-2 flex text-2xs text-secondary justify-between"
				><div>{min ? displayDate(new Date(min), true) : ''}</div>{#if max && min}<div
						class="hidden lg:block">{msToSec(max - min)}s</div
					>
				{/if}<div class="flex gap-1 items-center"
					>{max ? displayDate(new Date(max), true) : ''}{#if !max && min}{#if now}
							{msToSec(now - min)}s
						{/if}<Loader2 size={14} class="animate-spin" />{/if}</div
				></div
			>
		</div>
		{#each Object.values(flowModules) as k}
			<div class="px-2 py-2 grid grid-cols-12 w-full"
				><div>{k}</div>
				<div class="col-span-11 pt-1 px-2 flex min-h-6 w-full"
					>{#if min && total}
						<div class="flex flex-col gap-2 w-full">
							{#each items?.[k] ?? [] as b}
								<TimelineBar
									id={b?.id}
									{total}
									{min}
									started_at={b.started_at}
									len={b?.started_at ? b?.duration_ms ?? now - b?.started_at : 0}
									running={b?.duration_ms == undefined}
								/>
							{/each}
						</div>
					{/if}</div
				></div
			>
		{/each}
	</div>
{:else}
	<Loader2 class="animate-spin" />
{/if}
