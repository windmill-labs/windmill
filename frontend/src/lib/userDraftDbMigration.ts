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
import { getUsernameForNamespace } from './userNamespace'
import { randomUUID } from './utils/uuid'

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
 *
 * `path` is `''` for a legacy `/add` autosave: the new-item editor had
 * no path yet, so the key was just `.../{kind}/`. The caller mints a
 * fresh `u/{user}/draft_{uuid}` slot for those. A key without the
 * trailing `{kind}/` never matches and falls through to `undefined`, so
 * an empty `path` here always means the addable-slot case, not garbage.
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
			return { key, workspace, itemKind: kind, path }
		}
	}
	return undefined
}

/**
 * Mint a fresh `u/{user}/draft_{uuid}` slot for a pathless legacy `/add`
 * autosave, matching the convention the editors' `/add` redirects use
 * (`makeDraftAddLoad`). Underscores, not dashes — path segments are
 * `[a-zA-Z0-9_]` words and downstream consumers treat `-` as foreign —
 * and `randomUUID` rather than `crypto.randomUUID` (the WebCrypto version
 * is unavailable on non-secure origins, common for self-hosted).
 */
function mintDraftAddPath(): string {
	const username = getUsernameForNamespace()
	const uuid = randomUUID().replaceAll('-', '_')
	return `u/${username}/draft_${uuid}`
}

/**
 * Extract the payload that was stored in localStorage. The LS schema
 * was `{ value: V, lastWrittenAt?: number, remoteRev?: ..., ... }`.
 * The migration needs `value` plus `lastWrittenAt` (the LS copy's age,
 * passed as `last_sync` so a fresher server draft wins the upload).
 * Returns `undefined` when the slot is empty / unparseable / wrong
 * shape.
 */
function readPayload(key: string): { value: unknown; lastWrittenAt?: number } | undefined {
	try {
		const raw = localStorage.getItem(key)
		if (raw == null || raw === 'undefined') return undefined
		const parsed = JSON.parse(raw)
		if (parsed == null || typeof parsed !== 'object' || !('value' in parsed)) return undefined
		const lastWrittenAt = (parsed as { lastWrittenAt?: unknown }).lastWrittenAt
		return {
			value: (parsed as { value: unknown }).value,
			lastWrittenAt: typeof lastWrittenAt === 'number' ? lastWrittenAt : undefined
		}
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
 * push each to `POST /drafts/save_draft`, and remove it from LS once
 * settled. Entries that fail to migrate (parse error, network error,
 * unrecognized kind, ...) stay in LS and are retried on the next mount.
 *
 * The LS copy's `lastWrittenAt` rides along as `last_sync`, so the
 * server only accepts the upload when its own draft (if any) is not
 * fresher — the user may have kept editing the same path from another
 * browser after this one last wrote LS, and a blind overwrite would
 * destroy that newer server draft. Entries without `lastWrittenAt`
 * use epoch 0: insert when the slot is empty, yield to any existing
 * server draft. A `conflict` response means the server copy won — the
 * LS entry is dropped without uploading.
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
	const toMigrate: {
		key: string
		parsed: ParsedKey
		path: string
		value: unknown
		lastWrittenAt?: number
	}[] = []
	for (const key of keys) {
		const parsed = parseKey(key)
		if (!parsed) continue
		const payload = readPayload(key)
		if (payload === undefined) {
			// Unparseable or empty — clear so we don't keep retrying it.
			try {
				localStorage.removeItem(key)
			} catch {
				// ignore
			}
			continue
		}
		// A legacy `/add` autosave (`parsed.path === ''`) has no path of its
		// own — give it a fresh `u/{user}/draft_{uuid}` slot so it lands as a
		// regular draft-only item instead of being dropped.
		const path = parsed.path === '' ? mintDraftAddPath() : parsed.path
		toMigrate.push({
			key,
			parsed,
			path,
			value: payload.value,
			lastWrittenAt: payload.lastWrittenAt
		})
	}
	if (toMigrate.length === 0) return

	// Legacy drafts detected — tell the user the one-off upload is running.
	sendUserToast('Migrating local storage drafts ...', 'info')

	for (const { key, parsed, path, value, lastWrittenAt } of toMigrate) {
		try {
			const res = await DraftService.saveDraft({
				workspace: parsed.workspace,
				kind: parsed.itemKind,
				path,
				requestBody: { value, last_sync: new Date(lastWrittenAt ?? 0).toISOString() }
			})
			if (res.status === 'conflict') {
				console.info(
					`UserDraft LS→DB migration: server draft for ${path} is fresher, dropping LS copy`
				)
			}
			try {
				localStorage.removeItem(key)
			} catch {
				// Best-effort. If LS removal fails the next mount retries
				// the save (the conflict rule keeps it idempotent).
			}
		} catch (e) {
			// Leave the LS entry in place — the next mount tries again — but
			// surface it so the user isn't silently stuck, with an escape
			// hatch to drop the un-migratable draft.
			console.error('UserDraft LS→DB migration: failed for', key, e)
			sendUserToast(`Could not migrate draft ${path} in workspace ${parsed.workspace}`, 'error', [
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
			])
		}
	}
}
