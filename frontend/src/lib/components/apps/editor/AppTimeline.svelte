<script lang="ts">
	import { debounce, displayDate, msToSec } from '$lib/utils'
	import { onDestroy } from 'svelte'
	import { getDbClockNow } from '$lib/forLater'
	import { Loader2 } from 'lucide-svelte'
	import TimelineBar from '$lib/components/TimelineBar.svelte'

	export let jobs: {
		job: string
		component: string
		result?: string
		error?: string
		transformer?: { result?: string; error?: string }
		started_at?: number
		duration_ms?: number
	}[]

	let min: undefined | number = undefined
	let max: undefined | number = undefined
	let total: number | undefined = undefined

	let debounced = debounce(() => computeItems(jobs), 30)
	$: jobs && debounced()

	export function reset() {
		min = undefined
		max = undefined
	}

	function computeItems(
		jobs: {
			job: string
			component: string
			result?: string
			error?: string
			transformer?: { result?: string; error?: string }
			started_at?: number
			duration_ms?: number
		}[]
	): any {
		let nmin: undefined | number = undefined
		let nmax: undefined | number = undefined

		let isStillRunning = false

		jobs.forEach((v) => {
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

		min = nmin
		max = isStillRunning ? undefined : nmax
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
	{#each Object.values(jobs ?? []) as j}
		<div class="px-2 py-2 grid grid-cols-12 w-full"
			><div>{j.component}</div>
			<div class="col-span-11 pt-1 px-2 flex min-h-6 w-full"
				>{#if min && total}
					{#if j.started_at}
						<TimelineBar
							id={j.job}
							{total}
							{min}
							started_at={j.started_at}
							len={j.duration_ms ?? now - j.started_at}
							running={j.duration_ms == undefined}
						/>
					{/if}
				{/if}</div
			></div
		>
	{/each}
</div>
