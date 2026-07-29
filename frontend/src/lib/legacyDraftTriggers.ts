/**
 * One-shot conversion of the `draft_triggers` array that runnable drafts used to
 * carry.
 *
 * Before triggers had their own `draft` rows, the editors kept undeployed
 * trigger configs inside the runnable's own draft value. Those drafts are still
 * in the database, and nothing reads the field any more, so promote each entry
 * to a real `trigger_*` draft row on load — the trigger then behaves like any
 * other: it survives a reload, lists on the triggers pages, and deploys through
 * `deployDraft`.
 *
 * Entries of a kind with no draft row (native triggers, see `triggerDraftKind`)
 * have nowhere to go and are dropped. That combination — a native trigger added
 * in an editor, never deployed, from before the upgrade — has no representation
 * in the new model.
 *
 * Idempotent: the caller strips the field, so a converted draft has none.
 */
import { UserDraft, type UserDraftItemKind } from '$lib/userDraft.svelte'
import { newDraftTriggerPath, triggerDraftKind, type Trigger } from '$lib/components/triggers/utils'

export async function migrateLegacyDraftTriggers(opts: {
	legacy: Trigger[] | undefined
	runnablePath: string
	isFlow: boolean
	workspace: string
}): Promise<{ kind: UserDraftItemKind; path: string }[]> {
	const { legacy, runnablePath, isFlow, workspace } = opts
	if (!Array.isArray(legacy) || legacy.length === 0) return []

	const taken: string[] = []
	const migrated: { kind: UserDraftItemKind; path: string }[] = []
	for (const trigger of legacy) {
		const cfg = trigger?.draftConfig
		const kind = trigger?.type ? triggerDraftKind(trigger.type) : undefined
		if (!cfg || !kind) continue
		const path =
			typeof cfg.path === 'string' && cfg.path
				? cfg.path
				: newDraftTriggerPath(runnablePath, trigger.type, taken, !!trigger.isPrimary)
		taken.push(path)
		// `canSave` was the old editor's validity flag on the in-memory config; it
		// is not part of a trigger's config and must not reach the deploy call.
		const { canSave: _canSave, ...rest } = cfg
		UserDraft.save(
			kind,
			path,
			{ ...rest, path, script_path: cfg.script_path ?? runnablePath, is_flow: isFlow },
			{ workspace }
		)
		await UserDraft.forcePersist(kind, path, { workspace })
		migrated.push({ kind, path })
	}
	return migrated
}
