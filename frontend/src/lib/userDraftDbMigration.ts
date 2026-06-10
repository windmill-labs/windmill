/**
 * One-off migration from the localStorage-backed UserDraft autosave to
 * the new per-user DB-backed `draft` table.
 *
 * Runs AFTER `migrateLegacyUserDrafts` (which folds the original
 * `flow` / `app-{path}` / `rawapp-{path}` style keys into the
 * intermediate `userdraft/w/{workspace}/{kind}/{path}` format). This
 * file picks up from that intermediate format and pushes each entry
 * over to `POST /drafts/save_draft`, deleting it from localStorage
 * only after the POST returns successfully.
 *
 * Per-entry semantics:
 *   - A successful save deletes the source key, so on next page load
 *     it's gone. The migration is idempotent without a sentinel: any
 *     entry that failed (network error, parse error, ...) is simply
 *     left in place and retried on the next mount.
 *   - We do not gate on the current workspace — the key embeds its own
 *     workspace, and the auth token covers every workspace the user is
 *     a member of. Migrating only the active workspace would orphan
 *     entries for any other workspace the user had been editing in.
 *
 * Intentionally NOT importing from `userDraft.svelte.ts`: this is
 * one-way LS-clearing scaffolding, kept here so the runtime module
 * stays free of legacy decoders.
 */

import { DraftService } from './gen'
import type { UserDraftItemKind } from './gen'
import { sendUserToast } from './toast'

// Mirror of `USER_DRAFT_ITEM_KINDS` from `userDraft.svelte.ts`. Inlined
// here so the migration module can be imported without pulling in the
// reactive runtime. Tested by the type assertion below: the compiler
// rejects this file if a kind is added to the OpenAPI schema but not
// listed here (and vice-versa).
const ITEM_KINDS = [
	'script',
	'flow',
	'app',
	'raw_app',
	'resource',
	'variable',
	'trigger_schedule',
	'trigger_webhook',
	'trigger_default_email',
	'trigger_email',
	'trigger_http',
	'trigger_websocket',
	'trigger_postgres',
	'trigger_kafka',
	'trigger_nats',
	'trigger_mqtt',
	'trigger_sqs',
	'trigger_gcp',
	'trigger_azure',
	'trigger_poll',
	'trigger_cli',
	'trigger_nextcloud',
	'trigger_google',
	'trigger_github'
] as const satisfies readonly UserDraftItemKind[]

type _Exhaustive =
	Exclude<UserDraftItemKind, (typeof ITEM_KINDS)[number]> extends never ? true : never
const _: _Exhaustive = true
void _

const KEY_PREFIX = 'userdraft/w/'

type ParsedKey = {
	key: string
	workspace: string
	itemKind: UserDraftItemKind
	path: string
}

/**
 * Split a `userdraft/w/{workspace}/{kind}/{path}` key into its parts.
 * Returns `undefined` for keys that don't match the schema or whose
 * kind isn't one we recognize — we ignore those rather than risk
 * sending a junk POST.
 */
function parseKey(key: string): ParsedKey | undefined {
	if (!key.startsWith(KEY_PREFIX)) return undefined
	const rest = key.slice(KEY_PREFIX.length)
	const firstSlash = rest.indexOf('/')
	if (firstSlash <= 0) return undefined
	const workspace = rest.slice(0, firstSlash)
	const afterWorkspace = rest.slice(firstSlash + 1)
	for (const kind of ITEM_KINDS) {
		const kindPrefix = `${kind}/`
		if (afterWorkspace.startsWith(kindPrefix)) {
			const path = afterWorkspace.slice(kindPrefix.length)
			if (!path) return undefined
			return { key, workspace, itemKind: kind, path }
		}
	}
	return undefined
}

/**
 * Extract the payload that was stored in localStorage. The LS schema
 * was `{ value: V, lastWrittenAt?: number, remoteRev?: ..., ... }`.
 * For the migration we only need `value`. Returns `undefined` when the
 * slot is empty / unparseable / wrong shape.
 */
function readPayload(key: string): unknown {
	try {
		const raw = localStorage.getItem(key)
		if (raw == null || raw === 'undefined') return undefined
		const parsed = JSON.parse(raw)
		if (parsed == null || typeof parsed !== 'object' || !('value' in parsed)) return undefined
		return (parsed as { value: unknown }).value
	} catch {
		return undefined
	}
}

function collectKeys(): string[] {
	const keys: string[] = []
	for (let i = 0; i < localStorage.length; i++) {
		const k = localStorage.key(i)
		if (k != null && k.startsWith(KEY_PREFIX)) keys.push(k)
	}
	return keys
}

/**
 * Walk every `userdraft/w/{ws}/{kind}/{path}` entry in localStorage,
 * push each to `POST /drafts/save_draft` with `force: true`, and remove
 * it from LS on a successful response. Entries that fail to migrate
 * (parse error, network error, unrecognized kind, ...) stay in LS and
 * are retried on the next mount.
 *
 * Resolves only when every candidate has been attempted. Logs per-entry
 * errors but never throws — the caller is fire-and-forget.
 */
export async function migrateUserDraftsToDb(): Promise<void> {
	if (typeof localStorage === 'undefined') return
	const keys = collectKeys()
	if (keys.length === 0) return

	// Parse up front so we only announce the migration when there's a real
	// `userdraft/...` entry to upload (and can drop unparseable junk first).
	const toMigrate: { key: string; parsed: ParsedKey; value: unknown }[] = []
	for (const key of keys) {
		const parsed = parseKey(key)
		if (!parsed) continue
		const value = readPayload(key)
		if (value === undefined) {
			// Unparseable or empty — clear so we don't keep retrying it.
			try {
				localStorage.removeItem(key)
			} catch {
				// ignore
			}
			continue
		}
		toMigrate.push({ key, parsed, value })
	}
	if (toMigrate.length === 0) return

	// Legacy drafts detected — tell the user the one-off upload is running.
	sendUserToast('Migrating local storage drafts ...', 'info')

	for (const { key, parsed, value } of toMigrate) {
		try {
			await DraftService.saveDraft({
				workspace: parsed.workspace,
				kind: parsed.itemKind,
				path: parsed.path,
				requestBody: { value, force: true }
			})
			try {
				localStorage.removeItem(key)
			} catch {
				// Best-effort. If LS removal fails the next mount retries
				// the save (force: true is idempotent).
			}
		} catch (e) {
			// Leave the LS entry in place — the next mount tries again — but
			// surface it so the user isn't silently stuck, with an escape
			// hatch to drop the un-migratable draft.
			console.error('UserDraft LS→DB migration: failed for', key, e)
			sendUserToast(
				`Could not migrate draft ${parsed.path} in workspace ${parsed.workspace}`,
				'error',
				[
					{
						label: 'Delete draft',
						callback: () => {
							try {
								localStorage.removeItem(key)
							} catch {
								// ignore
							}
						}
					}
				]
			)
		}
	}
}
