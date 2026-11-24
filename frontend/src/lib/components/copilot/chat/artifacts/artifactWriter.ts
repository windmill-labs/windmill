import { randomUUID } from '$lib/utils/uuid'
import {
	currentVersion,
	getArtifact,
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
	/** Stored as `version`; `artifact` is the row this write stored. */
	| { outcome: 'saved'; artifact: PersistedArtifact; version: number }
	/** The change leaves the artifact as it was. */
	| { outcome: 'unchanged'; artifact: PersistedArtifact }
	/** Made on a version that is no longer the latest, so nothing was stored. `artifact` is the
	 * latest, for the change to be redone on. */
	| { outcome: 'stale'; artifact: PersistedArtifact; latest: number }
	/** This browser could not store it: its storage is full or unavailable. */
	| { outcome: 'unstored' }
	/** No such artifact in this browser, or it is of another session. */
	| { outcome: 'missing' }
	/** The session's plan, which the caller's posture may not write. */
	| { outcome: 'plan-locked' }
	/** A session holds one plan, and this one is taken. */
	| { outcome: 'plan-taken'; plan: PersistedArtifact }

/**
 * Every write of an artifact goes through here: a change is stored only when made on the latest
 * version, and numbered after it. The check runs again inside the write's transaction, where
 * another tab may have stored a later version since the change was made.
 */
export class ArtifactWriter {
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
		const write = await mutateArtifact<WriteResult>(id, (existing) => {
			if (locked()) return { refused: { outcome: 'plan-locked' } }
			if (existing) return { refused: { outcome: 'plan-taken', plan: existing } }
			return {
				edit: minted(
					{ id, sessionId, chatId: input.chatId, kind: input.kind ?? 'md', role: input.role },
					{ name, content: input.content }
				)
			}
		})
		return written(write)
	}

	/**
	 * Store a change made on version `madeOn` of an artifact, refused unless that is still its
	 * latest. Undefined `madeOn` is this browser's latest.
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
	 * the proposer saw none: creating it the first time.
	 */
	async proposePlan(
		sessionId: string,
		revision: { name: string; content: string; note: string },
		chatId: string | undefined,
		madeOn: number
	): Promise<WriteResult> {
		const id = planArtifactId(sessionId)
		const row = await getArtifact(id)
		if (row) {
			if (madeOn !== currentVersion(row)) {
				return { outcome: 'stale', artifact: row, latest: currentVersion(row) }
			}
			return this.#revise(row, revision)
		}
		const change = { name: revision.name.trim(), content: revision.content }
		const write = await mutateArtifact<WriteResult>(id, (existing) => {
			// A plan another tab put here since: stored over it unread, this proposal would pass for
			// one made on it.
			if (existing) {
				return {
					refused: { outcome: 'stale', artifact: existing, latest: currentVersion(existing) }
				}
			}
			return { edit: minted({ id, sessionId, chatId, kind: 'md', role: 'plan' }, change) }
		})
		return written(write)
	}

	/** The next version of `row`. */
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
		const write = await mutateArtifact<WriteResult>(row.id, (stored) => {
			if (!stored || !inScope(stored, scope)) return { refused: { outcome: 'missing' } }
			if (planLocked(stored, scope)) return { refused: { outcome: 'plan-locked' } }
			// Another tab stored a later version since `row` was read, which this change would
			// replace unseen.
			if (currentVersion(stored) !== currentVersion(row)) {
				return {
					refused: { outcome: 'stale', artifact: stored, latest: currentVersion(stored) }
				}
			}
			return { edit: revise(stored, next, currentVersion(row) + 1) }
		})
		return written(write)
	}
}

function inScope(a: PersistedArtifact, scope: WriteScope | undefined): boolean {
	return scope?.sessionId === undefined || a.sessionId === scope.sessionId
}

function planLocked(a: PersistedArtifact, scope: WriteScope | undefined): boolean {
	return scope?.canWritePlan?.() === false && isPlanArtifact(a, a.sessionId)
}

function written(write: ArtifactWrite<WriteResult>): WriteResult {
	if (write.refused) return write.refused
	if (!write.artifact) return { outcome: 'unstored' }
	return { outcome: 'saved', artifact: write.artifact, version: currentVersion(write.artifact) }
}

/** Version 1 of a new artifact. */
function minted(
	identity: Pick<PersistedArtifact, 'id' | 'sessionId' | 'chatId' | 'kind' | 'role'>,
	change: { name: string; content: string }
): ArtifactEdit {
	const now = Date.now()
	const artifact: PersistedArtifact = {
		...identity,
		name: change.name,
		content: change.content,
		createdAt: now,
		updatedAt: now,
		version: 1
	}
	return { artifact, snapshots: [snapshotOf(artifact, 1)] }
}

/**
 * Version `version` of an artifact, written over `existing` inside the write's transaction, and
 * the snapshots it produces.
 */
function revise(
	existing: PersistedArtifact,
	change: { name: string; content: string; note?: string; keepApproved?: boolean },
	version: number
): ArtifactEdit {
	const textChanged = change.content !== existing.content
	const artifact: PersistedArtifact = {
		...existing,
		name: change.name,
		content: change.content,
		// A rename carries it only from the latest version: renaming a proposal the user turned
		// down would otherwise promote it.
		approvedVersion:
			change.keepApproved &&
			existing.approvedVersion !== undefined &&
			(textChanged || existing.approvedVersion === currentVersion(existing))
				? version
				: existing.approvedVersion,
		updatedAt: Date.now(),
		version
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
