import type { AiArtifact, AiArtifactWithContent } from '$lib/gen'
import { randomUUID } from '$lib/utils/uuid'
import { fingerprint, type ArtifactServer } from './artifactServer'
import {
	currentVersion,
	getArtifact,
	getArtifactVersion,
	isPlanArtifact,
	mutateArtifact,
	planArtifactId,
	versionKey,
	type ArtifactEdit,
	type ArtifactKind,
	type ArtifactVersion,
	type ArtifactWrite,
	type PersistedArtifact
} from './artifactsDB'

export interface CreateArtifactInput {
	name: string
	content: string
	kind?: ArtifactKind
	role?: PersistedArtifact['role']
	chatId?: string
}

/** A change to an artifact's name, its text, or both. */
export interface ArtifactChange {
	name?: string
	content?: string
	/** What the change does, in the editor's words, which the version picker shows. */
	note?: string
	/** Carry an approval onto the version a change of the text makes. Opt-in, so forgetting it
	 * leaves a draft rather than marking one agreed; ignored when nothing was approved. */
	keepApproved?: boolean
}

export interface WriteScope {
	/** Refuse an artifact of another session. */
	sessionId?: string
	/** Asked inside the write as well, so a posture entered while it waited still decides it. */
	canWritePlan?: () => boolean
}

export type WriteResult =
	/** Stored as `version`. `artifact` is this browser's copy, past `version` when another tab
	 * stored a later version before this write reached the store. */
	| { outcome: 'saved'; artifact: PersistedArtifact; version: number; item?: AiArtifact }
	/** The change leaves the artifact as it was. */
	| { outcome: 'unchanged'; artifact: PersistedArtifact }
	/** Made on a version that is no longer the latest, so nothing was stored. `artifact` is the
	 * latest, which this browser now holds, for the change to be redone on. */
	| { outcome: 'stale'; artifact: PersistedArtifact; latest: number }
	/** Made on a version that is no longer the latest, which another browser wrote and the server
	 * copy does not show while an older one is pinned: nothing was stored, and this browser cannot
	 * take that version. */
	| { outcome: 'behind'; artifact: PersistedArtifact; latest: number }
	/** This browser could not store it: its storage is full or unavailable. `shared` when the
	 * server copy took it, so people with access see it all the same. */
	| { outcome: 'unstored'; shared: boolean }
	/** No such artifact in this browser, or it is in Recently deleted, or of another session. */
	| { outcome: 'missing' }
	/** The session's plan, which the caller's posture may not write. */
	| { outcome: 'plan-locked' }
	/** A session holds one plan, and this one is taken. */
	| { outcome: 'plan-taken'; plan: PersistedArtifact }

/** How many fingerprints of its latest versions an artifact keeps; its snapshots, which can say
 * the same of older ones, are fewer. */
const MAX_FINGERPRINTS = 100

/**
 * Every write of an artifact goes through here: a change is stored only when made on the latest
 * version, numbered after it, and handed to the server copy, when there is one, before this
 * browser stores it. The server copy refuses a change made on a version that is not its latest,
 * which is how one written in another of its owner's browsers is noticed; this browser then takes
 * that version where the server copy shows it, for the change to be redone on.
 *
 * A call to the server that fails rejects with its error, and nothing is stored: a version stored
 * here that the server never weighed would be the base of this browser's next write, which would
 * then replace one written elsewhere in between.
 */
export class ArtifactWriter {
	readonly #server: ArtifactServer
	readonly #home: () => string | undefined

	/** `home` is the workspace a new artifact belongs to, where its server copy goes. */
	constructor(server: ArtifactServer, home: () => string | undefined = () => undefined) {
		this.#server = server
		this.#home = home
	}

	/** A new artifact, which reaches the server once it is shared. */
	async create(
		sessionId: string,
		input: CreateArtifactInput,
		scope?: WriteScope
	): Promise<WriteResult> {
		const locked = () => input.role === 'plan' && scope?.canWritePlan?.() === false
		if (locked()) return { outcome: 'plan-locked' }
		// A plan's id is the session's, so a second one cannot be minted.
		const id = input.role === 'plan' ? planArtifactId(sessionId) : randomUUID()
		const name = input.name.trim()
		const hash = await fingerprint(name, input.content)
		const workspaceId = this.#home()
		const write = await mutateArtifact<WriteResult>(id, (existing) => {
			if (locked()) return { refused: { outcome: 'plan-locked' } }
			if (existing && isLive(existing))
				return { refused: { outcome: 'plan-taken', plan: existing } }
			const change = { name, content: input.content }
			return {
				edit: existing
					? revive(existing, change, hash, workspaceId)
					: minted(
							{
								id,
								sessionId,
								chatId: input.chatId,
								kind: input.kind ?? 'md',
								role: input.role,
								workspaceId
							},
							change,
							hash
						)
			}
		})
		return written(write, false)
	}

	/**
	 * Store a change made on version `madeOn` of an artifact, refused unless that is still its
	 * latest: in this browser, where another tab may have stored a later one, and on the server
	 * copy, where another of its owner's browsers may have. Undefined `madeOn` is this browser's
	 * latest.
	 */
	async edit(
		id: string,
		change: ArtifactChange,
		madeOn: number | undefined,
		scope?: WriteScope
	): Promise<WriteResult> {
		const row = await getArtifact(id)
		if (!row || !inScope(row, scope)) return { outcome: 'missing' }
		if (planLocked(row, scope)) return { outcome: 'plan-locked' }
		if (madeOn !== undefined && madeOn !== currentVersion(row)) {
			return { outcome: 'stale', artifact: row, latest: currentVersion(row) }
		}
		return this.#revise(row, change, scope)
	}

	/**
	 * Put a proposal into the session's one plan document, made on version `madeOn` of it, 0 when
	 * the proposer saw none: creating it the first time, and taking the place of one in Recently
	 * deleted.
	 */
	async proposePlan(
		sessionId: string,
		revision: { name: string; content: string; note: string },
		chatId: string | undefined,
		madeOn: number
	): Promise<WriteResult> {
		const id = planArtifactId(sessionId)
		const row = await getArtifact(id)
		if (row && isLive(row)) {
			if (madeOn !== currentVersion(row)) {
				return { outcome: 'stale', artifact: row, latest: currentVersion(row) }
			}
			return this.#revise(row, revision)
		}
		const change = { name: revision.name.trim(), content: revision.content, note: revision.note }
		const hash = await fingerprint(change.name, change.content)
		const workspaceId = this.#home()
		const write = await mutateArtifact<WriteResult>(id, (existing) => {
			// A plan another tab put here since: stored over it unread, this proposal would pass for
			// one made on it.
			if (existing && isLive(existing)) {
				return {
					refused: { outcome: 'stale', artifact: existing, latest: currentVersion(existing) }
				}
			}
			return {
				edit: existing
					? revive(existing, change, hash, workspaceId)
					: minted({ id, sessionId, chatId, kind: 'md', role: 'plan', workspaceId }, change, hash)
			}
		})
		return written(write, false)
	}

	/**
	 * This browser's copy of an artifact, caught up with the server's latest version when another
	 * of its owner's browsers wrote it: unless an older one is pinned, which keeps the latest off
	 * the server, or the server's latest is one of this browser's older versions, which the next
	 * write brings the server copy past.
	 */
	async catchUp(local: PersistedArtifact, copy: AiArtifactWithContent): Promise<PersistedArtifact> {
		if (!isLive(local) || !showsLatest(copy) || copy.latest_hash === undefined) return local
		if (
			copy.latest_hash === (await known(local)) ||
			(await holdsOlder(local, copy.latest_version, copy.latest_hash))
		) {
			return local
		}
		const workspace = local.workspaceId ?? this.#home()
		const caught = workspace ? await this.#adopt(local, copy, workspace) : undefined
		return caught && caught !== 'unstored' ? caught : local
	}

	/**
	 * Choose the version people with access see: `pinned` keeps them on `version` while newer ones
	 * are written, unpinned shows them the latest. Refused unless this browser's latest version is
	 * the server's, which a server copy behind this browser is brought up to first.
	 */
	async show(
		id: string,
		workspace: string,
		shown: { pinned: boolean; version: number }
	): Promise<WriteResult> {
		const row = await getArtifact(id)
		if (!row || !isLive(row)) return { outcome: 'missing' }
		const latest = currentVersion(row)
		const version = shown.pinned ? shown.version : latest
		const text =
			version === latest ? row : await getArtifactVersion(id, version).catch(() => undefined)
		if (!text) return { outcome: 'missing' }
		const base = await known(row)
		const ask = () =>
			this.#server.show(workspace, id, {
				pinned: shown.pinned,
				version,
				name: text.name,
				content: text.content,
				base_hash: base
			})
		let result = await ask()
		if (
			result.outcome === 'stale' &&
			result.latest_hash !== undefined &&
			result.latest_version !== undefined &&
			(await holdsOlder(row, result.latest_version, result.latest_hash))
		) {
			const pushed = await this.#server.write(workspace, id, nextVersion(row, result.latest_hash))
			result = pushed.outcome === 'saved' ? await ask() : pushed
		}
		if (result.outcome === 'saved') {
			return { outcome: 'saved', artifact: row, version, item: result.artifact }
		}
		if (result.outcome !== 'stale' || result.latest_version === undefined) {
			return { outcome: 'missing' }
		}
		return this.#reconcile(row, workspace, result.latest_version)
	}

	/**
	 * Put the artifact on the server as its owner's item, with this browser's latest version as the
	 * one people see, before its access list is first changed. A copy already there keeps its path,
	 * access list and versions, and takes this browser's latest when its own is one of this
	 * browser's older versions. Undefined when this browser does not hold the artifact.
	 */
	async publish(id: string, workspace: string): Promise<AiArtifact | undefined> {
		const row = await getArtifact(id)
		if (!row || !isLive(row)) return undefined
		const item = await this.#server.publish(workspace, {
			artifact_id: row.id,
			name: row.name,
			kind: row.kind,
			role: row.role,
			session_id: row.sessionId,
			version: currentVersion(row),
			content: row.content
		})
		if (
			item.latest_hash === undefined ||
			!(await holdsOlder(row, item.latest_version, item.latest_hash))
		) {
			return item
		}
		try {
			const pushed = await this.#server.write(workspace, id, nextVersion(row, item.latest_hash))
			return pushed.artifact ?? item
		} catch {
			return item
		}
	}

	/** The next version of `row`, handed to the server copy first. */
	async #revise(
		row: PersistedArtifact,
		change: ArtifactChange,
		scope?: WriteScope
	): Promise<WriteResult> {
		const next = {
			...change,
			name: change.name === undefined ? row.name : change.name.trim(),
			content: change.content ?? row.content
		}
		if (next.name === row.name && next.content === row.content) {
			return { outcome: 'unchanged', artifact: row }
		}
		const base = await known(row)
		const hash = await fingerprint(next.name, next.content)
		const sent = await this.#send(row, next, base)
		if ('outcome' in sent) return sent
		const version = sent.version ?? currentVersion(row) + 1
		const write = await mutateArtifact<WriteResult>(row.id, (stored) => {
			if (!stored || !inScope(stored, scope)) return { refused: { outcome: 'missing' } }
			if (planLocked(stored, scope)) return { refused: { outcome: 'plan-locked' } }
			// Another tab stored a later version since `row` was read. The server took this change,
			// so that one is this change, which the other tab took from the server, or was made on
			// it; without the server, storing this one would replace it unseen.
			if (currentVersion(stored) !== currentVersion(row)) {
				return {
					refused:
						sent.version !== undefined
							? { outcome: 'saved', artifact: stored, version: sent.version }
							: { outcome: 'stale', artifact: stored, latest: currentVersion(stored) }
				}
			}
			return { edit: revise(stored, next, version, hash, base) }
		})
		return written(write, sent.version !== undefined)
	}

	/**
	 * Hand the next version of `row` to its server copy before this browser stores it. Resolves the
	 * number the server took it under, none when the server holds no copy, or the answer for a
	 * change made on a version that is not the server's latest.
	 */
	async #send(
		row: PersistedArtifact,
		next: { name: string; content: string },
		base: string
	): Promise<{ version?: number } | WriteResult> {
		const workspace = row.workspaceId ?? this.#home()
		if (!workspace) return {}
		const write = (baseHash: string) =>
			this.#server.write(workspace, row.id, {
				base_hash: baseHash,
				version: currentVersion(row) + 1,
				name: next.name,
				content: next.content,
				session_id: row.sessionId
			})
		let result = await write(base)
		// A server copy behind this browser, at one of the versions this change was made after.
		if (
			result.outcome === 'stale' &&
			result.latest_hash !== undefined &&
			result.latest_version !== undefined &&
			(await holdsOlder(row, result.latest_version, result.latest_hash))
		) {
			result = await write(result.latest_hash)
		}
		if (result.outcome === 'saved') {
			return { version: result.latest_version ?? currentVersion(row) + 1 }
		}
		if (result.outcome !== 'stale' || result.latest_version === undefined) return {}
		return this.#reconcile(row, workspace, result.latest_version)
	}

	/**
	 * The answer for a change the server copy refused because its latest version is one this
	 * browser does not hold, once this browser has taken that version where the copy shows it.
	 */
	async #reconcile(
		row: PersistedArtifact,
		workspace: string,
		latest: number
	): Promise<WriteResult> {
		const copy = await this.#server.get(workspace, row.id).catch(() => undefined)
		if (!copy || !showsLatest(copy)) {
			return { outcome: 'behind', artifact: row, latest: copy?.latest_version ?? latest }
		}
		const caught = await this.#adopt(row, copy, workspace)
		if (caught === 'unstored') return { outcome: 'unstored', shared: false }
		if (!caught) return { outcome: 'missing' }
		return { outcome: 'stale', artifact: caught, latest: currentVersion(caught) }
	}

	/**
	 * This browser's copy once it holds `copy`, the server's latest version, as the version after
	 * `row`'s: under the server's number, or above it when this browser's own versions took that
	 * one, which the server copy is then renumbered to, so that a version has one number in every
	 * browser, which pinning names it by. The stored row is the answer when another tab moved the
	 * store past `row`, or it holds that version already.
	 */
	async #adopt(
		row: PersistedArtifact,
		copy: AiArtifactWithContent,
		workspace: string
	): Promise<PersistedArtifact | 'unstored' | undefined> {
		const version = Math.max(copy.latest_version, currentVersion(row) + 1)
		// The server's own fingerprint, so the next write is made on exactly what it holds.
		const hash = copy.latest_hash ?? (await fingerprint(copy.name, copy.content))
		const write = await mutateArtifact<{ held?: PersistedArtifact }>(row.id, (stored) => {
			if (!stored || !isLive(stored)) return { refused: {} }
			if (
				currentVersion(stored) !== currentVersion(row) ||
				(stored.name === copy.name && stored.content === copy.content)
			) {
				return { refused: { held: stored } }
			}
			const change = { name: copy.name, content: copy.content, note: 'Written in another browser' }
			return { edit: revise(stored, change, version, hash) }
		})
		if (write.refused) return write.refused.held
		if (!write.artifact) return 'unstored'
		if (version > copy.latest_version) {
			try {
				await this.#server.write(workspace, row.id, {
					base_hash: hash,
					version,
					name: copy.name,
					content: copy.content,
					session_id: row.sessionId
				})
			} catch (err) {
				console.error('Could not number the server copy of the artifact', err)
			}
		}
		return write.artifact
	}
}

function isLive(a: PersistedArtifact): boolean {
	return a.deletedAt === undefined
}

function inScope(a: PersistedArtifact, scope: WriteScope | undefined): boolean {
	return isLive(a) && (scope?.sessionId === undefined || a.sessionId === scope.sessionId)
}

function planLocked(a: PersistedArtifact, scope: WriteScope | undefined): boolean {
	return scope?.canWritePlan?.() === false && isPlanArtifact(a, a.sessionId)
}

/** Whether the server copy's text is its latest version's: an older one pinned keeps it off. */
function showsLatest(copy: AiArtifactWithContent): boolean {
	return copy.deleted_at === undefined && copy.shown_version === copy.latest_version
}

/** The fingerprint of this browser's latest version: the server's, for one taken from it. */
async function known(a: PersistedArtifact): Promise<string> {
	return a.fingerprints?.[currentVersion(a)] ?? (await fingerprint(a.name, a.content))
}

/** Whether `version`, older than `local`'s latest, is one this browser holds with fingerprint
 * `hash`: the server's latest is then a version this browser wrote on the way to its own. */
async function holdsOlder(
	local: PersistedArtifact,
	version: number,
	hash: string
): Promise<boolean> {
	if (version >= currentVersion(local)) return false
	const kept = local.fingerprints?.[version]
	if (kept !== undefined) return kept === hash
	const held = await getArtifactVersion(local.id, version).catch(() => undefined)
	return held !== undefined && (await fingerprint(held.name, held.content)) === hash
}

/** This browser's latest version of `a`, for a server copy whose own latest, `serverHash`, is
 * one of the versions it came after. */
function nextVersion(a: PersistedArtifact, serverHash: string) {
	return {
		base_hash: serverHash,
		version: currentVersion(a),
		name: a.name,
		content: a.content,
		session_id: a.sessionId
	}
}

function written(write: ArtifactWrite<WriteResult>, shared: boolean): WriteResult {
	if (write.refused) return write.refused
	if (!write.artifact) return { outcome: 'unstored', shared }
	return { outcome: 'saved', artifact: write.artifact, version: currentVersion(write.artifact) }
}

/** Version 1 of a new artifact. */
function minted(
	identity: Pick<
		PersistedArtifact,
		'id' | 'sessionId' | 'chatId' | 'kind' | 'role' | 'workspaceId'
	>,
	change: { name: string; content: string },
	hash: string
): ArtifactEdit {
	const now = Date.now()
	const artifact: PersistedArtifact = {
		...identity,
		name: change.name,
		content: change.content,
		createdAt: now,
		updatedAt: now,
		version: 1,
		fingerprints: { 1: hash }
	}
	return { artifact, snapshots: [snapshotOf(artifact, 1)] }
}

/**
 * A plan in Recently deleted, back for a new plan of its session to take its place: the plan id
 * is the session's, so the slot is still the deleted one's. Its versions carry on, which keeps
 * the server copy taking new ones, but not its approval. That copy stays in the trash, unshared
 * until shared again.
 */
function revive(
	plan: PersistedArtifact,
	change: { name: string; content: string; note?: string },
	hash: string,
	workspaceId: string | undefined
): ArtifactEdit {
	const back: PersistedArtifact = {
		...plan,
		deletedAt: undefined,
		approvedVersion: undefined,
		workspaceId: plan.workspaceId ?? workspaceId
	}
	if (back.name === change.name && back.content === change.content) {
		return { artifact: { ...back, updatedAt: Date.now() }, snapshots: [] }
	}
	return revise(back, change, currentVersion(plan) + 1, hash)
}

/**
 * Version `version` of an artifact, with fingerprint `hash`, written over `existing` inside the
 * write's transaction, and the snapshots it produces. `base` is the fingerprint of `existing`'s
 * latest, when known.
 */
function revise(
	existing: PersistedArtifact,
	change: { name: string; content: string; note?: string; keepApproved?: boolean },
	version: number,
	hash: string,
	base?: string
): ArtifactEdit {
	const textChanged = change.content !== existing.content
	const artifact: PersistedArtifact = {
		...existing,
		name: change.name,
		content: change.content,
		// Carried by a change of the text only: a rename would otherwise promote a proposal the
		// user turned down.
		approvedVersion:
			change.keepApproved && textChanged && existing.approvedVersion !== undefined
				? version
				: existing.approvedVersion,
		updatedAt: Date.now(),
		version,
		fingerprints: latestOf({
			...existing.fingerprints,
			...(base === undefined ? {} : { [currentVersion(existing)]: base }),
			[version]: hash
		})
	}
	// An artifact written before history existed has no snapshot of its current content, and
	// once this write stamps `version` nothing would recognise it as one.
	const snapshots: ArtifactVersion[] =
		existing.version === undefined ? [snapshotOf(existing, currentVersion(existing))] : []
	snapshots.push(
		snapshotOf(artifact, version, change.note ?? (textChanged ? undefined : 'Renamed'))
	)
	return { artifact, snapshots }
}

function latestOf(fingerprints: Record<number, string>): Record<number, string> {
	const kept = Object.keys(fingerprints)
		.map(Number)
		.sort((a, b) => b - a)
		.slice(0, MAX_FINGERPRINTS)
	return Object.fromEntries(kept.map((v) => [v, fingerprints[v]]))
}

export function snapshotOf(a: PersistedArtifact, version: number, note?: string): ArtifactVersion {
	return {
		key: versionKey(a.id, version),
		artifactId: a.id,
		version,
		name: a.name,
		content: a.content,
		savedAt: a.updatedAt,
		note
	}
}
