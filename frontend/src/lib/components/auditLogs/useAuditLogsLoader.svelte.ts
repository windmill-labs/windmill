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

	function load(batchSize?: number): CancelablePromise<void> {
		pendingLoad?.cancel()

		const a = args()
		if (a.workspace == undefined && a.scope !== 'instance') {
			return CancelablePromiseUtils.pure<void>(undefined)
		}
		const total = Math.max(1, a.perPage)
		const size = Math.min(Math.max(1, batchSize ?? total), total)
		const isBatched = size < total
		// Rows are ordered by descending id, so batches after the first one use the last id as a
		// keyset cursor. Only the batch the page starts on needs an offset, and `page` can only
		// express offsets that are multiples of `size`: land on the closest one below and drop the
		// rows that come before the page.
		const startOffset = (Math.max(1, a.pageIndex) - 1) * total
		const firstPage = Math.floor(startOffset / size) + 1
		const skipFirst = startOffset - (firstPage - 1) * size

		loading = true
		hasMore = false
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
				hasMore = acc.length >= total
				if (isBatched) {
					batchProgress = { loaded: acc.length, total }
				}
				if (rows.length < size || acc.length >= total) {
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
		if (!isBatched && total > SMALL_BATCH_SIZE) {
			promise = CancelablePromiseUtils.onTimeout(promise, 4000, () => {
				sendUserToast('Loading audit logs is taking longer than expected...', 'warning', [
					{
						label: `Stream by batches of ${SMALL_BATCH_SIZE}`,
						callback: () => restreamWithBatchSize(SMALL_BATCH_SIZE)
					}
				])
			})
		}
		promise = CancelablePromiseUtils.finallyDo(promise, () => {
			if (slowLoadIntervalId) clearInterval(slowLoadIntervalId)
		})
		// Only on success: a cancel means another load already owns these.
		promise = CancelablePromiseUtils.pipe(promise, () => {
			batchProgress = null
			currentBatchSize = null
		})
		promise = CancelablePromiseUtils.catchErr(promise, (e) => {
			if (e instanceof CancelError) return CancelablePromiseUtils.pure<void>(undefined)
			loading = false
			batchProgress = null
			currentBatchSize = null
			sendUserToast(
				'There was an issue loading audit logs, see browser console for more details',
				true
			)
			console.error(e)
			return CancelablePromiseUtils.pure<void>(undefined)
		})
		pendingLoad = promise
		return promise
	}

	function restreamWithBatchSize(batchSize: number) {
		load(batchSize)
	}

	function stopBatchLoading() {
		pendingLoad?.cancel()
		pendingLoad = undefined
		batchProgress = null
		currentBatchSize = null
		loading = false
	}

	$effect(() => {
		const a = args()
		;[
			a.workspace,
			a.scope,
			a.username,
			a.operation,
			a.resource,
			a.actionKind,
			a.before,
			a.after,
			a.pageIndex,
			a.perPage
		]
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
