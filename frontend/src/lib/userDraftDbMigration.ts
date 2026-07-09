/**
 * One-off migration from the localStorage UserDraft autosave to the
 * DB-backed `draft` table. Reads the workspace-scoped
 * `userdraft/w/{workspace}/{kind}/{path}` keys (written by the editor during
 * the interim LS-backed phase, so the embedded workspace is correct), POSTing
 * each to `/drafts/update` and clearing the source key only on success — so
 * it's idempotent without a sentinel; failed entries retry next mount. Not
 * workspace-gated: keys embed their own workspace and the token covers all of
 * them, so gating would orphan other-workspace entries.
 *
 * Before uploading, each draft is compared against its deployed version
 * (script / flow / app); a draft that's deep-equal to what's deployed carries
 * no changes, so it's dropped instead of migrated (no error).
 */

import { AppService, DraftService, FlowService, ScriptService } from './gen'
import type { UserDraftItemKind } from './gen'
import type { App } from './components/apps/types'
import { migrateApp } from './components/apps/migrateApp'
import { sendUserToast } from './toast'
import { draftValuesEqual } from './userDraft.svelte'
import {
	openDraftMigrationErrorModal,
	reportDraftMigrationError
} from './userDraftMigrationErrors.svelte'
import { getUsernameForNamespace } from './userNamespace'
import { randomUUID } from './utils/uuid'

// Mirror of `USER_DRAFT_ITEM_KINDS`, inlined to avoid importing the reactive
// runtime. The `_Exhaustive` assertion below fails compilation if this list
// drifts from the OpenAPI schema's kinds.
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
	'trigger_github',
	'data_pipeline'
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
 * Split a `userdraft/w/{workspace}/{kind}/{path}` key into its parts, or
 * `undefined` for keys that don't match or have an unrecognized kind.
 * `path` is `''` only for a legacy `/add` autosave (key ended `.../{kind}/`);
 * the caller mints a fresh slot for those. An empty `path` here is always that
 * case, never garbage, since a non-matching key returns `undefined`.
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
 * autosave, matching the editors' `/add` redirect convention (`makeDraftAddLoad`).
 * Underscores not dashes (path segments are `[a-zA-Z0-9_]` words); `randomUUID`
 * not `crypto.randomUUID` (WebCrypto is absent on non-secure origins).
 */
function mintDraftAddPath(): string {
	const username = getUsernameForNamespace()
	const uuid = randomUUID().replaceAll('-', '_')
	return `u/${username}/draft_${uuid}`
}

/**
 * Extract `value` and `lastWrittenAt` from the LS payload (`{ value, lastWrittenAt?, ... }`).
 * `lastWrittenAt` rides along as `last_sync` so a fresher server draft wins.
 * `undefined` when the slot is empty / unparseable / wrong shape.
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

/**
 * Fetch the deployed value for a draft so the migration can drop a draft that
 * carries no changes (deep-equal to what's already deployed) instead of
 * uploading a no-op that would light up the "unsaved" badge. Returns the
 * comparable deployed payload, or `undefined` when there's nothing to compare
 * against: an unsupported kind, a pathless (minted `/add`) draft, or a fetch
 * miss (404 — the path is draft-only, so the draft is genuinely new). `getDraft`
 * is forced off so we compare against the deployed baseline, not our own draft.
 */
async function fetchDeployedValue(
	workspace: string,
	kind: UserDraftItemKind,
	path: string
): Promise<unknown | undefined> {
	if (!path) return undefined
	try {
		switch (kind) {
			case 'script':
				return await ScriptService.getScriptByPath({ workspace, path, getDraft: false })
			case 'flow':
				return await FlowService.getFlowByPath({ workspace, path, getDraft: false })
			case 'app': {
				// The app autosave stores the inner `App`, not the `AppWithLastVersion`
				// wrapper getAppByPath returns — compare against `.value`. Run
				// `migrateApp` so the deployed value matches the editor-migrated draft
				// (AppEditor `migrateApp`s `stateApp` on mount); without this an app
				// whose deployed row predates those field migrations never dedups.
				const app = await AppService.getAppByPath({ workspace, path, getDraft: false })
				const value = (app as { value?: App }).value
				if (value) migrateApp(value)
				return value
			}
			default:
				return undefined
		}
	} catch {
		// No deployed item at this path (or the fetch failed) — nothing to dedup
		// against, so the caller proceeds to upload the draft.
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
 * Push every LS `userdraft/...` entry to `/drafts/update`, clearing each on
 * success; failures stay in LS and retry next mount.
 *
 * `lastWrittenAt` rides as `last_sync` so the server rejects the upload when its
 * own draft is fresher (the user may have edited the same path from another
 * browser since this LS write). Missing `lastWrittenAt` uses epoch 0 (insert if
 * absent, else yield). A `conflict` response means the server won; drop the LS
 * copy. Never throws — the caller is fire-and-forget.
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
		// A legacy `/add` autosave has no path — mint a fresh slot so it lands
		// as a regular draft-only item instead of being dropped.
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

	// Legacy drafts detected — tell the user the one-off upload is running, with
	// an escape hatch to the modal where any failures show up as they happen.
	sendUserToast('Migrating local storage drafts ...', 'info', [
		{ label: 'See more', callback: openDraftMigrationErrorModal }
	])

	for (const { key, parsed, path, value, lastWrittenAt } of toMigrate) {
		try {
			// Dedup: if the draft is deep-equal to the deployed version it carries
			// no changes — drop it (no error) instead of uploading a no-op draft.
			// Fetches against `parsed.path` (the real item path); minted `/add`
			// drafts have `parsed.path === ''` and so are never deduped.
			const deployed = await fetchDeployedValue(parsed.workspace, parsed.itemKind, parsed.path)
			if (deployed !== undefined && draftValuesEqual(value, deployed)) {
				try {
					localStorage.removeItem(key)
				} catch {
					// Best-effort; a stale LS entry is harmless — it re-dedups next mount.
				}
				continue
			}
			// Preserve the draft's original age: stamp `created_at` with the LS
			// write time (epoch 0 when unknown) so migrated drafts don't all
			// resurface to the top as freshly created. Same value as `last_sync`,
			// which still drives the conflict check.
			const writtenAt = new Date(lastWrittenAt ?? 0).toISOString()
			const res = await DraftService.updateDraft({
				workspace: parsed.workspace,
				kind: parsed.itemKind,
				path,
				requestBody: { value, last_sync: writtenAt, created_at: writtenAt }
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
			reportDraftMigrationError({
				key,
				workspace: parsed.workspace,
				itemKind: parsed.itemKind,
				path,
				value
			})
		}
	}
}
