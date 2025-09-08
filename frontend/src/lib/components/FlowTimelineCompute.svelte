<script lang="ts">
	import { debounce, readFieldsRecursively } from '$lib/utils'
	import { onDestroy, untrack } from 'svelte'
	import { getDbClockNow } from '$lib/forLater'

	interface Props {
		flowModules: string[]
		durationStatuses: Record<
			string,
			{
				byJob: Record<string, { created_at?: number; started_at?: number; duration_ms?: number }>
			}
		>
		flowDone?: boolean
		min: undefined | number
		max: undefined | number
		total: number | undefined
		items:
			| Record<
					string,
					Array<{ created_at?: number; started_at?: number; duration_ms?: number; id: string }>
			  >
			| undefined
		now: number
	}

	let {
		flowModules,
		durationStatuses,
		flowDone = false,
		min = $bindable(),
		max = $bindable(),
		total = $bindable(),
		items = $bindable(),
		now = $bindable(getDbClockNow().getTime())
	}: Props = $props()

	let { debounced, clearDebounce } = debounce(() => computeItems(durationStatuses), 30)
	$effect(() => {
		readFieldsRecursively(durationStatuses)
		flowDone != undefined && durationStatuses && untrack(() => debounced())
	})

	export function reset() {
		min = undefined
		max = undefined
		items = computeItems(durationStatuses)
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
