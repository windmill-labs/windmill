<script lang="ts">
	import { debounce } from '$lib/utils'
	import { getContext, onDestroy } from 'svelte'

	import TimelineBar from '$lib/components/TimelineBar.svelte'
	import type { AppViewerContext } from '../types'

	const { jobs, jobsById } = getContext<AppViewerContext>('AppViewerContext')

	let min: undefined | number = undefined
	let max: undefined | number = undefined
	let total: number | undefined = undefined

	let debounced = debounce(() => computeItems($jobs), 30)
	$: $jobs && $jobsById && debounced()

	let items: Record<
		string,
		{ started_at?: number; started_compute_at?: number; duration_ms?: number; id: string }[]
	> = {}

	export function reset() {
		min = undefined
		max = undefined
	}

	function computeItems(jobs: string[]): any {
		let nmin: undefined | number = undefined
		let nmax: undefined | number = undefined

		let isStillRunning = false

		let nitems: Record<
			string,
			{ started_at?: number; started_compute_at?: number; duration_ms?: number; id: string }[]
		> = {}
		jobs.forEach((k) => {
			let v = $jobsById[k]
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
				if (v.started_compute_at && v.duration_ms) {
					let lmax = v.started_compute_at + v.duration_ms
					if (!nmax) {
						nmax = lmax
					} else {
						nmax = Math.max(nmax, lmax)
					}
				}
			}
			if (!nitems[v.component]) {
				nitems[v.component] = []
			}
			nitems[v.component].push({
				started_at: v.started_at,
				duration_ms: v.duration_ms,
				started_compute_at: v.started_compute_at,
				id: v.job
			})
		})

		Object.values(nitems).forEach((v) => {
			v.sort((x, y) => {
				if (!x.started_at) {
					return -1
				} else if (!y.started_at) {
					return 1
				} else {
					return x.started_at - y.started_at
				}
			})
		})

		min = nmin
		max = isStillRunning ? undefined : nmax
		if (max && min) {
			total = max - min
		}
		items = nitems
	}

	let now = Date.now()

	let interval = setInterval((x) => {
		if (!max) {
			now = Date.now()
		}
		if (min && (!max || total == undefined)) {
			total = max ? max - min : Math.max(now - min, 2000)
		}
	}, 30)

	onDestroy(() => {
		interval && clearInterval(interval)
	})
</script>

<!-- <pre class="text-xs">
{JSON.stringify(items, null, 4)}
</pre> -->
<div class="divide-y">
	<div class="flex flex-row-reverse mb-2 items-center text-sm text-secondary px-2">
		<div class="flex gap-4 items-center">
			<div class="flex gap-2 items-center">
				<div>Waiting for executor</div>
				<div class="h-4 w-4 bg-gray-500" />
			</div>

			<div class="flex gap-2 items-center">
				<div>Execution</div>
				<div class="h-4 w-4 bg-blue-500/90" />
			</div>
		</div>
	</div>
	{#each Object.entries(items) as [k, v]}
		<div class="px-2 py-2 grid grid-cols-12 w-full"
			><div class="col-span-2">{k}</div>
			<div class="col-span-10 pt-1 px-2 flex min-h-6 w-full"
				>{#if min && total}
					<div class="flex flex-col gap-2 w-full">
						{#each v ?? [] as b}
							{@const waitingLen = b?.started_at
								? b.started_compute_at
									? b.started_compute_at - b?.started_at
									: now - b?.started_at
								: 0}
							<div class="flex w-full">
								<TimelineBar
									id={b?.id}
									{total}
									{min}
									gray
									started_at={b.started_at}
									len={waitingLen < 100 ? 0 : waitingLen - 100}
									running={b?.started_compute_at == undefined}
								/>
								{#if b.started_compute_at}
									<TimelineBar
										id={b?.id}
										{total}
										{min}
										concat
										started_at={b.started_compute_at}
										len={b.started_compute_at ? b?.duration_ms ?? now - b?.started_compute_at : 0}
										running={b?.duration_ms == undefined}
									/>
								{/if}
							</div>
						{/each}
					</div>
				{/if}</div
			></div
		>
	{/each}
</div>
