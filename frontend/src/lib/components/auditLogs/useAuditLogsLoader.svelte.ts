import { untrack } from 'svelte'
import { AuditService, CancelError, CancelablePromise, type AuditLog } from '$lib/gen'
import { CancelablePromiseUtils } from '$lib/cancelable-promise-utils'
import { sendUserToast } from '$lib/toast'
import type { ActionKind } from '$lib/common'

export interface AuditLogsLoaderArgs {
	workspace: string | undefined
	scope: undefined | 'all_workspaces' | 'instance'
	username: string
	operation: string
	resource: string | undefined
	actionKind: ActionKind | 'all'
	before: string | undefined
	after: string | undefined
	pageIndex: number
	perPage: number
}

const SMALL_BATCH_SIZE = 25
// The page size comes from the url, and a batched load turns it into one request per batch, so it
// has to be capped at the largest size the page itself offers.
const MAX_PER_PAGE = 1000

/**
 * Where the first batch of a page starts. Rows are ordered by descending id, so the batches after
 * it follow a `before_id` cursor and only this one needs an offset. `page` can only express
 * offsets that are multiples of `batchSize`: land on the closest one at or below the page start,
 * and report how many rows of that batch belong to the previous page.
 */
export function computeFirstBatch(
	pageIndex: number,
	perPage: number,
	batchSize: number
): { firstPage: number; skipFirst: number } {
	const startOffset = (Math.max(1, pageIndex) - 1) * perPage
	const firstPage = Math.floor(startOffset / batchSize) + 1
	return { firstPage, skipFirst: startOffset - (firstPage - 1) * batchSize }
}

/**
 * Loads one page of audit logs, optionally streaming it in smaller batches so rows show up as
 * they arrive on instances where a full page takes a long time to come back.
 */
export function useAuditLogsLoader(args: () => AuditLogsLoaderArgs) {
	let logs: AuditLog[] | undefined = $state()
	let loading = $state(false)
	let hasMore = $state(false)
	let batchProgress = $state<{ loaded: number; total: number } | null>(null)
	let currentBatchSize = $state<number | null>(null)

	let pendingLoad: CancelablePromise<void> | undefined
	let pendingLoadHasRows = false

	function fetchBatch(
		a: AuditLogsLoaderArgs,
		page: number,
		limit: number,
		beforeId: number | undefined
	): CancelablePromise<AuditLog[]> {
		return AuditService.listAuditLogs({
			workspace: a.scope === 'instance' ? 'global' : a.workspace!,
			page,
			perPage: limit,
			beforeId,
			before: a.before,
			after: a.after,
			username: a.username === 'all' ? undefined : a.username,
			operation: a.operation === 'all' || a.operation === '' ? undefined : a.operation,
			resource: a.resource === 'all' || a.resource === '' ? undefined : a.resource,
			actionKind: a.actionKind === 'all' ? undefined : a.actionKind,
			allWorkspaces: a.scope === 'all_workspaces'
		})
	}

	function clearBatchState() {
		batchProgress = null
		currentBatchSize = null
	}

	/**
	 * A load that stops or fails never completed its page: it says nothing about whether a next
	 * page exists, and with no rows of its own the rows of the query it replaced would stand in
	 * for its result.
	 */
	function abandonLoad() {
		if (!pendingLoadHasRows) {
			logs = []
		}
		hasMore = false
		clearBatchState()
		loading = false
	}

	function load(batchSize?: number): CancelablePromise<void> {
		pendingLoad?.cancel()
		pendingLoad = undefined
		pendingLoadHasRows = false

		const a = args()
		if (a.workspace == undefined && a.scope !== 'instance') {
			loading = false
			clearBatchState()
			return CancelablePromiseUtils.pure<void>(undefined)
		}
		const total = Math.min(Math.max(1, Math.floor(a.perPage) || 1), MAX_PER_PAGE)
		const size = Math.min(Math.max(1, batchSize ?? total), total)
		const isBatched = size < total
		const { firstPage, skipFirst } = computeFirstBatch(a.pageIndex, total, size)

		loading = true
		batchProgress = isBatched ? { loaded: 0, total } : null
		currentBatchSize = isBatched ? size : null

		const acc: AuditLog[] = []
		let slowBatchToastShown = false

		function loadBatch(beforeId: number | undefined, skip: number): CancelablePromise<void> {
			let fetch = fetchBatch(a, beforeId === undefined ? firstPage : 1, size, beforeId)
			if (isBatched && size > 1) {
				fetch = CancelablePromiseUtils.onTimeout(fetch, 4000, () => {
					if (slowBatchToastShown) return
					slowBatchToastShown = true
					sendUserToast(
						`Streaming by batches of ${size} is slow, try loading one at a time`,
						'warning',
						[{ label: 'Stream 1 by 1', callback: () => restreamWithBatchSize(1) }]
					)
				})
			}
			return CancelablePromiseUtils.then(fetch, (rows) => {
				acc.push(...(skip > 0 ? rows.slice(skip) : rows).slice(0, total - acc.length))
				logs = [...acc]
				loading = false
				pendingLoadHasRows = true
				if (isBatched) {
					batchProgress = { loaded: acc.length, total }
				}
				if (rows.length < size || acc.length >= total) {
					// Only once the page is complete: a half-streamed page says nothing about
					// whether there is a next one.
					hasMore = acc.length >= total
					return CancelablePromiseUtils.pure<void>(undefined)
				}
				return loadBatch(rows[rows.length - 1].id, 0)
			})
		}

		let slowLoadIntervalId: ReturnType<typeof setInterval> | undefined
		if (isBatched) {
			slowLoadIntervalId = setInterval(() => {
				sendUserToast(
					'Loading is taking a long time...',
					'warning',
					[{ label: 'Stop loading', callback: () => stopBatchLoading() }],
					undefined,
					8000
				)
			}, 15000)
		}

		let promise = loadBatch(undefined, skipFirst)
		if (!isBatched) {
			promise = CancelablePromiseUtils.onTimeout(promise, 4000, () => {
				const smaller = total > SMALL_BATCH_SIZE ? SMALL_BATCH_SIZE : 1
				sendUserToast(
					'Loading audit logs is taking longer than expected...',
					'warning',
					total > 1
						? [
								{
									label: smaller === 1 ? 'Stream 1 by 1' : `Stream by batches of ${smaller}`,
									callback: () => restreamWithBatchSize(smaller)
								}
							]
						: []
				)
			})
		}
		promise = CancelablePromiseUtils.finallyDo(promise, () => {
			if (slowLoadIntervalId) clearInterval(slowLoadIntervalId)
		})
		// Only on success: a cancel means another load already owns these.
		promise = CancelablePromiseUtils.pipe(promise, clearBatchState)
		promise = CancelablePromiseUtils.catchErr(promise, (e) => {
			if (e instanceof CancelError) return CancelablePromiseUtils.pure<void>(undefined)
			abandonLoad()
			sendUserToast(
				'There was an issue loading audit logs, see browser console for more details',
				true
			)
			console.error(e)
			return CancelablePromiseUtils.pure<void>(undefined)
		})
		const thisLoad = promise
		// The "Stop loading" toast outlives the load it was raised for, so a settled load has to
		// stop being the pending one.
		CancelablePromiseUtils.pipe(thisLoad, () => {
			if (pendingLoad === thisLoad) pendingLoad = undefined
		})
		pendingLoad = thisLoad
		return thisLoad
	}

	function restreamWithBatchSize(batchSize: number) {
		load(batchSize)
	}

	function stopBatchLoading() {
		if (!pendingLoad) return
		pendingLoad.cancel()
		pendingLoad = undefined
		abandonLoad()
	}

	$effect(() => {
		// Building the args reads every filter, which is what registers this effect's dependencies.
		args()
		untrack(() => load())
		return () => {
			pendingLoad?.cancel()
			pendingLoad = undefined
		}
	})

	return {
		reload: () => load(),
		restreamWithBatchSize,
		stopBatchLoading,
		get logs() {
			return logs
		},
		get loading() {
			return loading
		},
		get hasMore() {
			return hasMore
		},
		get batchProgress() {
			return batchProgress
		},
		get currentBatchSize() {
			return currentBatchSize
		}
	}
}
