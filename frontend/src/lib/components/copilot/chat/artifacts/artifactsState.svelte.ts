import { randomUUID } from '$lib/utils/uuid'
import {
	currentVersion,
	deleteArtifact,
	getArtifact,
	getArtifactVersion,
	isPlanArtifact,
	listArtifactVersions,
	listArtifactsForSession,
	mutateArtifact,
	planArtifactId,
	versionKey,
	type ArtifactEdit,
	type ArtifactKind,
	type ArtifactVersion,
	type PersistedArtifact
} from './artifactsDB'

export interface CreateArtifactInput {
	name: string
	content: string
	kind?: ArtifactKind
	role?: PersistedArtifact['role']
	approvedVersion?: number
	chatId?: string
}

export interface UpdateArtifactInput {
	name?: string
	content?: string
	/** Recorded on the snapshot this update produces; ignored if content is unchanged. */
	note?: string
	/** The version that stands as the agreed plan. Set explicitly only on approval. */
	approvedVersion?: number
	/** Carry an existing approval onto the version this write produces. Opt-in, so forgetting
	 * it leaves a draft rather than marking one agreed; ignored when nothing was approved. */
	keepApproved?: boolean
}

/** A session holds one plan, and this one is taken. Carries the document that holds it,
 * because the only useful thing a caller can do next is revise that one. */
export class PlanSlotTakenError extends Error {
	constructor(readonly plan: PersistedArtifact) {
		super(`Session ${plan.sessionId} already has a plan document`)
		this.name = 'PlanSlotTakenError'
	}
}

/**
 * A write that would have touched the session's plan, refused by the policy its caller passed.
 *
 * Thrown rather than returned as `undefined`, which already means "no such artifact here": a
 * caller that conflates the two answers a bad id with "the plan is read-only" and sends the
 * model to fix a problem it does not have.
 */
export class PlanWriteRefusedError extends Error {
	constructor() {
		super('The session plan document may not be written here')
		this.name = 'PlanWriteRefusedError'
	}
}

export class ArtifactPersistenceError extends Error {
	constructor() {
		super('The plan document could not be saved')
		this.name = 'ArtifactPersistenceError'
	}
}

/**
 * Reactive view of the active session's artifacts, owned by AIChatManager (like
 * AttachedFilesStore). The consumer drives which session is loaded via setSession(); the
 * write tools mutate through create/update/remove, which persist and update the in-memory
 * list in one step.
 */
export class SessionArtifactsStore {
	artifacts = $state<PersistedArtifact[]>([])
	loading = $state(false)

	#sessionId: string | undefined
	// A later load always wins, even if an earlier DB read resolves after it.
	#seq = 0

	/** Load the given session's artifacts into the reactive list, if it changed. */
	async setSession(sessionId: string | undefined): Promise<void> {
		// Skip same-id resyncs: in-memory owns the loaded session, so a DB reload would
		// drop artifacts whose best-effort persist failed.
		if (sessionId === this.#sessionId) return
		this.#sessionId = sessionId
		await this.#load()
	}

	/** Re-read the loaded session's artifacts from the store, for records another
	 *  tab wrote after this one loaded. Forces the read setSession skips: that
	 *  skip protects local edits whose best-effort persist failed, while a tab
	 *  catching up on another tab's finished turn wants the store's truth. */
	async resyncFromStore(): Promise<void> {
		if (this.#sessionId === undefined) return
		await this.#load()
	}

	async #load(): Promise<void> {
		const token = ++this.#seq
		const id = this.#sessionId
		if (!id) {
			this.artifacts = []
			this.loading = false
			return
		}
		this.loading = true
		const items = await listArtifactsForSession(id)
		if (token !== this.#seq) return
		this.artifacts = sortByUpdatedDesc(items)
		this.loading = false
	}

	// Bump #seq so an in-flight #load (snapshot taken before this write) can't clobber it;
	// that load early-returns without clearing loading, so clear it here.
	#applyWrite(next: PersistedArtifact[]): void {
		this.#seq++
		this.artifacts = next
		this.loading = false
	}

	// Insert-or-replace: `update` resolves from the database too, so a write can be the first
	// this session hears of a plan another tab created.
	#reflect(artifact: PersistedArtifact): void {
		if (artifact.sessionId !== this.#sessionId) return
		const rest = this.artifacts.filter((a) => a.id !== artifact.id)
		this.#applyWrite(sortByUpdatedDesc([artifact, ...rest]))
	}

	async get(id: string): Promise<PersistedArtifact | undefined> {
		// In-memory first: a write whose persist silently failed (quota) is still readable here.
		return this.artifacts.find((a) => a.id === id) ?? (await getArtifact(id))
	}

	async listForSession(sessionId: string): Promise<PersistedArtifact[]> {
		if (sessionId === this.#sessionId) return [...this.artifacts]
		return sortByUpdatedDesc(await listArtifactsForSession(sessionId))
	}

	/**
	 * Persist a new artifact for `sessionId` and reflect it in the list if that session is loaded.
	 *
	 * `opts.canWritePlan` is asked at the mutation point rather than sampled by the caller: a
	 * posture entered while this write was waiting on the store still gets to decide it.
	 */
	async create(
		sessionId: string,
		input: CreateArtifactInput,
		opts?: { canWritePlan?: () => boolean }
	): Promise<PersistedArtifact> {
		const now = Date.now()
		const draft = (id: string): PersistedArtifact => ({
			id,
			sessionId,
			chatId: input.chatId,
			kind: input.kind ?? 'md',
			role: input.role,
			approvedVersion: input.approvedVersion,
			name: input.name,
			content: input.content,
			createdAt: now,
			updatedAt: now,
			version: 1
		})
		// A plan's id is the session's, so a second one cannot be minted; the slot check happens
		// on the row this write is about to replace, inside the transaction that replaces it.
		const id = input.role === 'plan' ? planArtifactId(sessionId) : randomUUID()
		const { outcome, artifact } = await mutateArtifact(id, (existing) => {
			// Before the slot check: a posture that may not mint a plan at all is the more useful
			// thing to say, and it holds whether or not the session already has one.
			if (input.role === 'plan' && opts?.canWritePlan?.() === false) {
				throw new PlanWriteRefusedError()
			}
			if (existing) throw new PlanSlotTakenError(existing)
			const created = draft(id)
			return { artifact: created, snapshots: [snapshotOf(created, 1)] }
		})
		// An ordinary artifact degrades unpersisted; a plan cannot. Returning one the database
		// refused would let the user approve a plan that disappears on reload.
		if (input.role === 'plan' && outcome !== 'saved') throw new ArtifactPersistenceError()
		const written = artifact ?? draft(id)
		this.#reflect(written)
		return written
	}

	/**
	 * Merge changes into an existing artifact. Returns undefined if `id` is unknown, or if
	 * `opts.sessionId` is given and the artifact belongs to a different session.
	 */
	async update(
		id: string,
		input: UpdateArtifactInput,
		opts?: { sessionId?: string; canWritePlan?: () => boolean }
	): Promise<PersistedArtifact | undefined> {
		let refused = false
		const { outcome, artifact } = await mutateArtifact(id, (stored) => {
			// Read inside the mutator: hoisted out, it would weigh a stale copy against a fresh one.
			const held = this.artifacts.find((a) => a.id === id)
			const existing = furtherAlong(stored, held)
			if (!existing || (opts?.sessionId !== undefined && existing.sessionId !== opts.sessionId)) {
				refused = true
				return undefined
			}
			// A plan mark on *either* candidate is enough: furtherAlong can settle on a copy whose
			// role is unset, and rows whose marks disagree are what this guards.
			if (
				opts?.canWritePlan?.() === false &&
				[stored, held].some((a) => a !== undefined && isPlanArtifact(a, a.sessionId))
			) {
				throw new PlanWriteRefusedError()
			}
			return reviseInto(existing, input)
		})
		if (refused) return undefined
		if (artifact?.role === 'plan' && outcome !== 'saved') throw new ArtifactPersistenceError()
		if (artifact) this.#reflect(artifact)
		return artifact
	}

	/**
	 * Put a proposal into the session's one plan document, creating it the first time.
	 *
	 * Both halves inside one transaction, so a second tab proposing at the same moment revises
	 * the row this one wrote rather than racing it: the id is the session's, and whichever
	 * transaction runs second reads the first one's result.
	 */
	async savePlan(
		sessionId: string,
		revision: { name: string; content: string; note: string },
		chatId: string | undefined
	): Promise<PersistedArtifact> {
		const id = planArtifactId(sessionId)
		const { outcome, artifact } = await mutateArtifact(id, (existing) => {
			if (existing) return reviseInto(existing, revision)
			const now = Date.now()
			const created: PersistedArtifact = {
				id,
				sessionId,
				chatId,
				kind: 'md',
				role: 'plan',
				name: revision.name,
				content: revision.content,
				createdAt: now,
				updatedAt: now,
				version: 1
			}
			return { artifact: created, snapshots: [snapshotOf(created, 1)] }
		})
		// A plan the database refused would let the user approve one that disappears on reload.
		if (!artifact || outcome !== 'saved') throw new ArtifactPersistenceError()
		this.#reflect(artifact)
		return artifact
	}

	/**
	 * Stamp the version the user agreed to, and nothing else.
	 *
	 * Not `update`: that rebuilds the row, so an approval computed while another tab was
	 * revising would carry this tab's older content back over the newer text. Read and
	 * patched in one transaction, it can only ever move the pointer.
	 */
	async approve(id: string, version: number): Promise<boolean> {
		// No version number this could name, and a stamped NaN would leave a pointer that no
		// comparison in planVersionView can ever match.
		if (!Number.isSafeInteger(version) || version < 1) return false
		const { outcome, artifact } = await mutateArtifact(
			id,
			(existing, snapshot) => {
				if (!existing) return undefined
				// Only a plan carries an approval, and only a version still readable is worth
				// pointing at: the bar offering "view the plan you approved" has to land somewhere.
				// The current version needs no snapshot — one written before history existed has none.
				if (!isPlanArtifact(existing, existing.sessionId)) return undefined
				if (version !== currentVersion(existing) && !snapshot) return undefined
				return { artifact: { ...existing, approvedVersion: version }, snapshots: [] }
			},
			{ readVersion: version }
		)
		// Reflected only once it is stored, unlike an ordinary edit, which degrades unpersisted:
		// content the store lost is still content, but an approval the store lost never happened,
		// and showing the `plan` pill over it would put the user's name on it anyway.
		if (!artifact || outcome !== 'saved') return false
		this.#reflect(artifact)
		return true
	}

	/**
	 * Every snapshot of an artifact, newest first. Empty if `id` is unknown, or if
	 * `opts.sessionId` is given and the artifact belongs to a different session — snapshots
	 * carry the document's full text, so this scopes like update() rather than trusting
	 * every caller to check first.
	 */
	async listVersions(id: string, opts?: { sessionId?: string }): Promise<ArtifactVersion[]> {
		const artifact = await this.get(id)
		if (opts?.sessionId !== undefined && artifact?.sessionId !== opts.sessionId) return []
		const stored = await listArtifactVersions(id)
		if (!artifact) return stored
		const version = currentVersion(artifact)
		// An artifact written before history existed has no snapshot of its current
		// content, so stand one in — the picker must never show a document as absent
		// from its own history.
		if (!stored.some((v) => v.version === version)) {
			return [snapshotOf(artifact, version), ...stored]
		}
		return stored
	}

	/** One snapshot; scoped by `opts.sessionId` like listVersions when it is given. */
	async getVersion(
		id: string,
		version: number,
		opts?: { sessionId?: string }
	): Promise<ArtifactVersion | undefined> {
		const artifact = await this.get(id)
		if (opts?.sessionId !== undefined && artifact?.sessionId !== opts.sessionId) return undefined
		// The current content is held in memory, so it is the one version a broken store can
		// still serve; for any older one the read failing propagates (see getArtifactVersion).
		const live =
			artifact && currentVersion(artifact) === version ? snapshotOf(artifact, version) : undefined
		try {
			return (await getArtifactVersion(id, version)) ?? live
		} catch (err) {
			if (live) return live
			throw err
		}
	}

	async remove(id: string): Promise<void> {
		await deleteArtifact(id)
		// Guard on presence: a no-op remove must not invalidate an in-flight load.
		const next = this.artifacts.filter((a) => a.id !== id)
		if (next.length !== this.artifacts.length) this.#applyWrite(next)
	}
}

/**
 * Neither copy is authoritative: only the store carries an edit another tab made, and only
 * memory carries one the store refused (quota) and `update` handed back unpersisted.
 * Always preferring one side reverts the other's text on the next edit, so the later wins.
 */
function furtherAlong(
	stored: PersistedArtifact | undefined,
	held: PersistedArtifact | undefined
): PersistedArtifact | undefined {
	if (!stored || !held) return stored ?? held
	const heldVersion = currentVersion(held)
	const storedVersion = currentVersion(stored)
	// A rename earns no version, so at equal versions only the clock separates the two.
	// Both are written by the same browser, which is what makes the stamps comparable.
	if (heldVersion === storedVersion) return held.updatedAt > stored.updatedAt ? held : stored
	return heldVersion > storedVersion ? held : stored
}

/**
 * The next version of an artifact, and the snapshots that edit produces. Shared by every
 * writer so the version and approval rules cannot drift between them; always given the row
 * `furtherAlong` settled on, never a remembered one.
 */
function reviseInto(existing: PersistedArtifact, input: UpdateArtifactInput): ArtifactEdit {
	// Only a content change earns a version: a rename or an identical rewrite would otherwise
	// fill the picker with entries the user cannot tell apart.
	const contentChanged = input.content !== undefined && input.content !== existing.content
	const version = currentVersion(existing) + (contentChanged ? 1 : 0)
	// Carried onto a version this write produced, so one that produces none moves nothing: a
	// rename would otherwise promote a proposal the user turned down.
	const approvedVersion =
		input.approvedVersion ??
		(input.keepApproved && existing.approvedVersion !== undefined && contentChanged
			? version
			: existing.approvedVersion)
	const artifact: PersistedArtifact = {
		...existing,
		name: input.name ?? existing.name,
		content: input.content ?? existing.content,
		approvedVersion,
		updatedAt: Date.now(),
		version
	}
	const snapshots: ArtifactVersion[] = []
	// An artifact written before history existed has no snapshot of its current content, so
	// capture one on *any* update, not just a content change: this write stamps `version`, and
	// nothing afterwards would recognise it as pre-history.
	if (existing.version === undefined) {
		snapshots.push(snapshotOf(existing, currentVersion(existing)))
	}
	if (contentChanged) {
		snapshots.push(snapshotOf(artifact, version, input.note))
	}
	return { artifact, snapshots }
}

function snapshotOf(a: PersistedArtifact, version: number, note?: string): ArtifactVersion {
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

function sortByUpdatedDesc(items: PersistedArtifact[]): PersistedArtifact[] {
	return [...items].sort((a, b) => b.updatedAt - a.updatedAt)
}

/**
 * The plan first, the rest still newest-first. Display order only — `list_artifacts` reads
 * the store's own order. A partition rather than a lift of one row, so it survives a session
 * that briefly holds two.
 */
export function planFirst(items: PersistedArtifact[]): PersistedArtifact[] {
	if (!items.some((a) => a.role === 'plan')) return items
	return [...items.filter((a) => a.role === 'plan'), ...items.filter((a) => a.role !== 'plan')]
}
