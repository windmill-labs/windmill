import { debounce, readFieldsRecursively } from '$lib/utils'
import { untrack } from 'svelte'
import { getDbClockNow } from '$lib/forLater'
import type { DurationStatus } from './components/graph/model'

export type TimelineItems = Record<
	string,
	Array<{ created_at?: number; started_at?: number; duration_ms?: number; id: string }>
>

export class TimelineCompute {
	#flowModules = $state<string[]>([])
	#durationStatuses = $state<Record<string, DurationStatus>>({})
	#flowDone = $state(false)
	#interval: number | undefined = undefined
	#debounceInstance: { debounced: () => void; clearDebounce: () => void }

	min = $state<number | undefined>(undefined)
	max = $state<number | undefined>(undefined)
	total = $state<number | undefined>(undefined)
	items = $state<TimelineItems | undefined>(undefined)
	now = $state<number>(getDbClockNow().getTime())

	constructor(
		flowModules: string[],
		durationStatuses: Record<string, DurationStatus>,
		flowDone: boolean = false
	) {
		this.#flowModules = flowModules
		this.#durationStatuses = durationStatuses
		this.#flowDone = flowDone

		this.#debounceInstance = debounce(() => this.computeItems(this.#durationStatuses), 30)

		// Set up reactivity using $effect
		$effect(() => {
			readFieldsRecursively(this.#durationStatuses)
			this.#flowDone != undefined &&
				this.#durationStatuses &&
				untrack(() => this.#debounceInstance.debounced())
		})

		// Set up interval for updating now and total for running jobs
		this.#interval = setInterval(() => {
			if (!this.max) {
				this.now = getDbClockNow().getTime()
			}
			if (this.min && (!this.max || this.total == undefined)) {
				this.total = this.max ? this.max - this.min : Math.max(this.now - this.min, 2000)
			}
		}, 30)
	}

	reset() {
		this.min = undefined
		this.max = undefined
		this.#flowDone = false
		this.items = this.computeItems(this.#durationStatuses)
	}

	updateInputs(
		flowModules: string[],
		durationStatuses: Record<string, DurationStatus>,
		flowDone: boolean = false
	) {
		this.#flowModules = flowModules
		this.#durationStatuses = durationStatuses
		this.#flowDone = flowDone
	}

	setFlowDone(flowDone: boolean) {
		this.#flowDone = flowDone
	}

	destroy() {
		if (this.#interval) {
			clearInterval(this.#interval)
		}
		this.#debounceInstance.clearDebounce()
	}

	private computeItems(
		durationStatuses: Record<
			string,
			{
				byJob: Record<string, { created_at?: number; started_at?: number; duration_ms?: number }>
			}
		>
	): TimelineItems {
		let nmin: undefined | number = undefined
		let nmax: undefined | number = undefined

		let isStillRunning = false

		let cnt = 0
		let nitems: TimelineItems = {}
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
				if (!this.#flowDone && v.duration_ms == undefined) {
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
		this.items = nitems
		this.min = nmin

		this.max =
			isStillRunning || (cnt < this.#flowModules.length && !this.#flowDone) ? undefined : nmax
		if (this.max && this.min) {
			this.total = this.max - this.min
			this.total = Math.max(this.total, 2000)
		}

		return nitems
	}
}
