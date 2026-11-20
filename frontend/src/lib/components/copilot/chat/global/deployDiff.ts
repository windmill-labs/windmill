import type { UserDraftItemKind } from '$lib/gen'
import { canonicalDraftSideValue, getDraftDiffValues } from '$lib/utils_draft_deploy'
import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
import { cleanValueProperties, replaceFalseWithUndefined } from '$lib/utils'
import {
	computeDiffParts,
	expireWorkspaceDiffList,
	resolveWorkspaceDiffTarget
} from './diffSnapshot'
import { itemKindFor, readLocalDraftCellByKind } from './userDraftAdapter'
import type { TriggerKind, WorkspaceItemType } from './workspaceItems'

export interface DeployDiff {
	kind: UserDraftItemKind
	/** Where the deploy lands: a draft created in an editor is stored at a synthetic
	 *  `u/<user>/draft_<uuid>` key, which the model may pass, with its chosen path in the draft. */
	path: string
	deployed: unknown
	draft: unknown
	/** Never deployed: this deploy creates the item. */
	isNew: boolean
	added: number
	removed: number
}

/** Deployed vs draft for the target of a pending `deploy_workspace_item` call, in the shape
 *  DiffDrawer's simple mode takes. Resolves the draft the way the `diff` tool does: a
 *  friendly path to its storage row, and a parked autosave flushed with the autosave toggle
 *  honored, so a read never persists edits the user kept local. Those are shown instead,
 *  since the deploy flushes them before it deploys. */
export async function loadDeployDiff(
	workspace: string,
	args: { type?: WorkspaceItemType; path?: string; trigger_kind?: TriggerKind }
): Promise<DeployDiff> {
	const { type, path } = args
	const itemKind = type && path ? itemKindFor(type, args.trigger_kind) : undefined
	if (!type || !path || !itemKind) {
		throw new Error('The deploy call does not name a draft')
	}
	const kinds: UserDraftItemKind[] = type === 'app' ? ['raw_app', 'app'] : [itemKind]
	const target = await resolveWorkspaceDiffTarget(workspace, kinds, path)
	if (!target) {
		throw new Error(`You have no draft at ${path}`)
	}
	const query = { workspace, itemKind: target.kind, path: target.storagePath }
	await UserDraftDbSyncer.flush(query, { honorAutosaveToggle: true })
	expireWorkspaceDiffList(workspace)

	const values = await getDraftDiffValues(
		target.kind,
		target.storagePath,
		workspace,
		target.draftOnly
	)
	const unsaved =
		UserDraftDbSyncer.hasUnsavedDisabledChanges(query) ||
		UserDraftDbSyncer.getState(query).state === 'failed' ||
		UserDraftDbSyncer.getConflict(query).conflict
	const localCell = unsaved
		? readLocalDraftCellByKind(workspace, target.kind, target.storagePath)
		: undefined
	const draft =
		localCell !== undefined ? canonicalDraftSideValue(target.kind, localCell) : values.draft
	const isNew = target.draftOnly || values.noDeployed
	// Cleaned as DiffDrawer cleans its sides, or row markers such as `is_draft: false` count as
	// changes the drawer then does not show.
	const clean = (v: unknown) => cleanValueProperties(replaceFalseWithUndefined(v))
	const parts = computeDiffParts(
		isNew ? undefined : clean(values.deployed),
		clean(draft),
		'deployed',
		'draft'
	)
	const counts = { added: 0, removed: 0 }
	for (const patch of [parts.patch, ...Object.values(parts.files ?? {}).map((f) => f.patch)]) {
		for (const line of patch.split('\n')) {
			if (line.startsWith('+') && !line.startsWith('+++')) counts.added++
			else if (line.startsWith('-') && !line.startsWith('---')) counts.removed++
		}
	}
	// Read from the local cell before canonicalizing: the raw-app canonical value drops `draft_path`.
	type Staged = { draft_path?: string; path?: string } | null | undefined
	const local = localCell as Staged
	const saved = draft as Staged
	const landsAt =
		(target.draftOnly
			? (local?.draft_path ?? local?.path ?? saved?.draft_path ?? saved?.path)
			: undefined) ?? path
	return { kind: target.kind, path: landsAt, deployed: values.deployed, draft, isNew, ...counts }
}
