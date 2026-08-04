/**
 * Bulk operations behind the Home page's selection bar. Each runner performs
 * exactly what the matching per-row menu entry does, once per selected item, so
 * a batch can never take a path the single-row action wouldn't.
 *
 * Delete and discard stay distinct: discarding a draft on a deployed item only
 * removes the draft, discarding a draft-only item removes the item, and deleting
 * addresses the deployed row. A draft-only item is therefore not deletable —
 * there is nothing deployed at its path.
 */
import { AppService, FlowService, ScriptService } from '$lib/gen'
import { updateItemPathAndSummary } from '$lib/components/moveRenameManager'
import { discardDraft } from '$lib/utils_draft_deploy'
import type { BulkItem } from './homeSelection.svelte'

export type BulkAction = 'move' | 'archive' | 'unarchive' | 'delete' | 'discard'

export type BulkContext = {
	workspace: string
	/** Deleting a script is admin-only, mirroring the row menu. */
	isAdmin: boolean
}

/** Why `action` cannot be applied to `item`; `undefined` means it can. The
 * string is surfaced verbatim, so it has to say whether the limit comes from
 * permissions, the item's kind, or its state. */
export function blockedReason(
	action: BulkAction,
	item: BulkItem,
	ctx: BulkContext
): string | undefined {
	const notOwner = 'you are not an owner of this path'
	switch (action) {
		case 'move':
			if (item.draftOnly) return 'a draft-only item has no deployed path to move'
			if (item.archived) return 'archived items cannot be moved'
			if (!item.owner) return notOwner
			if (!item.canWrite) return 'you do not have write permission on this path'
			return undefined
		case 'archive':
		case 'unarchive':
			if (item.kind !== 'script' && item.kind !== 'flow')
				return 'only scripts and flows can be archived'
			if (item.draftOnly) return 'a draft-only item has nothing deployed to archive'
			if (!item.owner) return notOwner
			if (!item.canWrite) return 'you do not have write permission on this path'
			if (action === 'archive' && item.archived) return 'already archived'
			if (action === 'unarchive' && !item.archived) return 'not archived'
			return undefined
		case 'delete':
			if (item.draftOnly) return 'a draft-only item is removed by discarding its draft'
			if (item.kind === 'script' && !ctx.isAdmin)
				return 'deleting a script requires being a workspace admin'
			if (item.kind === 'flow' && !item.owner) return notOwner
			if (!item.canWrite) return 'you do not have write permission on this path'
			return undefined
		case 'discard':
			if (!item.isDraft && !item.draftOnly) return 'no draft of yours on this item'
			return undefined
	}
}

export function eligible(action: BulkAction, items: BulkItem[], ctx: BulkContext): BulkItem[] {
	return items.filter((i) => blockedReason(action, i, ctx) == undefined)
}

/** Where an item lands under `target` (`f/<folder>` or `u/<user>`): everything
 * below its own owner prefix is preserved, so nested paths keep their shape. */
export function movedPath(item: BulkItem, target: string): string {
	const rest = item.path.split('/').slice(2).join('/')
	return `${target}/${rest}`
}

async function moveItem(ctx: BulkContext, item: BulkItem, target: string): Promise<void> {
	const newPath = movedPath(item, target)
	// Re-saving a script at its current path would mint a pointless new version.
	if (newPath === item.path) return
	await updateItemPathAndSummary({
		workspace: ctx.workspace,
		kind: item.kind,
		initialPath: item.path,
		newPath,
		newSummary: item.summary
	})
}

async function setArchived(ctx: BulkContext, item: BulkItem, archived: boolean): Promise<void> {
	if (item.kind === 'flow') {
		await FlowService.archiveFlowByPath({
			workspace: ctx.workspace,
			path: item.path,
			requestBody: { archived }
		})
		return
	}
	if (archived) {
		await ScriptService.archiveScriptByPath({ workspace: ctx.workspace, path: item.path })
		return
	}
	// Scripts have no unarchive route: redeploying the current version as its own
	// child clears the flag, exactly as the row menu does.
	const r = await ScriptService.getScriptByPath({ workspace: ctx.workspace, path: item.path })
	await ScriptService.createScript({
		workspace: ctx.workspace,
		requestBody: { ...r, parent_hash: r.hash, lock: r.lock }
	})
}

async function deleteItem(ctx: BulkContext, item: BulkItem): Promise<void> {
	if (item.kind === 'script') {
		await ScriptService.deleteScriptByPath({ workspace: ctx.workspace, path: item.path })
	} else if (item.kind === 'flow') {
		await FlowService.deleteFlowByPath({ workspace: ctx.workspace, path: item.path })
	} else {
		await AppService.deleteApp({ workspace: ctx.workspace, path: item.path })
	}
}

async function discardItemDraft(ctx: BulkContext, item: BulkItem): Promise<void> {
	// The draft overlay is the one place a raw app is its own kind.
	const kind = item.kind === 'app' && item.rawApp ? 'raw_app' : item.kind
	// invalidate=false: the caller refreshes the draft list once for the batch.
	const res = await discardDraft(kind, item.path, ctx.workspace, item.draftOnly, false, false)
	if (!res.success) throw new Error(res.error ?? 'discard failed')
}

export type BulkOutcome = { item: BulkItem; error?: string }

/**
 * Apply `action` to each item in turn, never aborting the batch on a failure —
 * every item gets its own outcome so partial success is reported rather than
 * hidden. `onProgress` is called after each item with the number completed.
 */
export async function runBulk(
	action: BulkAction,
	items: BulkItem[],
	ctx: BulkContext,
	opts: { target?: string; onProgress?: (done: number) => void } = {}
): Promise<BulkOutcome[]> {
	const outcomes: BulkOutcome[] = []
	for (const item of items) {
		try {
			switch (action) {
				case 'move':
					if (!opts.target) throw new Error('no target folder')
					await moveItem(ctx, item, opts.target)
					break
				case 'archive':
					await setArchived(ctx, item, true)
					break
				case 'unarchive':
					await setArchived(ctx, item, false)
					break
				case 'delete':
					await deleteItem(ctx, item)
					break
				case 'discard':
					await discardItemDraft(ctx, item)
					break
			}
			outcomes.push({ item })
		} catch (e: any) {
			outcomes.push({ item, error: e?.body ?? e?.message ?? String(e) })
		}
		opts.onProgress?.(outcomes.length)
	}
	return outcomes
}
