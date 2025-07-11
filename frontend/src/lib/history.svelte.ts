import { deepEqual } from 'fast-equals'
import { get, writable, type Writable } from 'svelte/store'

export type History<T> = Writable<{ history: T[]; index: number }>

export function initHistory<T>(initial: T): History<T> {
	return writable({ history: [$state.snapshot(initial) as T], index: 0 })
}

export function undo<T>(history: History<T> | undefined, now: T): T {
	if (history) {
		let chistory = get(history)
		if (chistory.index == 0) return now
		if (!deepEqual(chistory.history[chistory.index], now)) {
			push(history, now, true)
		} else {
			history.update((history) => {
				history.index--
				return history
			})
		}
		let nhistory = get(history)
		let r = nhistory.history[nhistory.index]
		if (!r) {
			console.log('undo failed', nhistory, now)
			return now
		} else {
			return r
		}
	} else {
		return now
	}
}

export function redo<T>(history: History<T> | undefined): T {
	if (history) {
		history?.update((history) => {
			if (history.index < history.history.length - 1) {
				history.index++
			}
			return history
		})
		let nhistory = get(history)
		return nhistory.history[nhistory.index]
	} else {
		return undefined as T
	}
}

export function push<T>(history: History<T> | undefined, value: T, noSetIndex: boolean = false) {
	history?.update((history) => {
		history.history = structuredClone(history.history.slice(0, history.index + 1))
		const toPush = $state.snapshot(value)
		history.history.push(toPush as T)
		if (!noSetIndex) {
			history.index = history.history.length - 1
		}
		if (history.history.length > 20) {
			history.history = history.history.slice(history.history.length - 20)
			history.index -= 1
		}
		return history
	})
}
