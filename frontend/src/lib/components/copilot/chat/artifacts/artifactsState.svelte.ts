import { randomUUID } from '$lib/utils/uuid'
import { AiService, type AiArtifactWithContent, type AiArtifactWriteResult } from '$lib/gen'
import {
	currentVersion,
	getArtifact,
	getArtifactVersion,
	isPlanArtifact,
	listArtifactVersions,
	listArtifactsForSession,
	mutateArtifact,
	patchArtifact,
	planArtifactId,
	versionKey,
	type ArtifactEdit,
	type ArtifactKind,
	type ArtifactVersion,
	type PersistedArtifact
} from './artifactsDB'
import { homeWorkspace, publishArtifact, restoreArtifact, trashArtifact } from './artifactLibrary'

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

/** A new version of an artifact, not stored because a later one exists: one its owner wrote
 * from another browser since this one last did, against which the server refused it, or one
 * another tab stored after this edit was made. */
export class ArtifactStaleError extends Error {
	constructor(
		readonly artifact: PersistedArtifact,
		/** The latest version: the server's, or the one another tab stored. */
		readonly latest: number,
		/** Whether this browser now holds that version, for the edit to be redone on it. */
		readonly caughtUp: boolean,
		/** The version the server took the edit as, before another tab stored a later one. That
		 * one was made on it, since the server takes no write made on an older version, so the
		 * edit is already in it and redoing it would apply it twice. */
		readonly savedAs?: number
	) {
		super(`Artifact ${artifact.id} changed in another tab or browser (now v${latest})`)
		this.name = 'ArtifactStaleError'
	}
}

/**
 * Reactive view of the active session's artifacts, owned by AIChatManager (like
 * AttachedFilesStore). The consumer drives which session is loaded via setSession(); the
 * write tools mutate through create/update/trash, which persist and update the in-memory
 * list in one step.
 */
export class SessionArtifactsStore {
	artifacts = $state<PersistedArtifact[]>([])
	loading = $state(false)

	#sessionId: string | undefined
	// A later load always wins, even if an earlier DB read resolves after it.
	#seq = 0
	#workspace: (() => string | undefined) | undefined

	/** `workspace` is the one the session works in; a store without one reads artifacts and
	 * never reaches the server. */
	constructor(workspace?: () => string | undefined) {
		this.#workspace = workspace
	}

	/** The workspace a new artifact of this session belongs to. */
	#home(): string | undefined {
		const ws = this.#workspace?.()
		return ws ? homeWorkspace(ws) : undefined
	}

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
		this.artifacts = sortByUpdatedDesc(items.filter(isLive))
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

	/** An artifact of any session, unless it is in Recently deleted. */
	async get(id: string): Promise<PersistedArtifact | undefined> {
		// In-memory first: a write whose persist silently failed (quota) is still readable here.
		const found = this.artifacts.find((a) => a.id === id) ?? (await getArtifact(id))
		return found && isLive(found) ? found : undefined
	}

	async listForSession(sessionId: string): Promise<PersistedArtifact[]> {
		if (sessionId === this.#sessionId) return [...this.artifacts]
		return sortByUpdatedDesc((await listArtifactsForSession(sessionId)).filter(isLive))
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
		const workspaceId = this.#home()
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
			version: 1,
			workspaceId
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
			if (existing && isLive(existing)) throw new PlanSlotTakenError(existing)
			if (existing) {
				const edit = reviseInto(revived(existing), { name: input.name, content: input.content })
				edit.artifact.workspaceId ??= workspaceId
				return edit
			}
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
		const prior = await this.get(id)
		const admitted =
			prior !== undefined &&
			(opts?.sessionId === undefined || prior.sessionId === opts.sessionId) &&
			!(opts?.canWritePlan?.() === false && isPlanArtifact(prior, prior.sessionId))
		const sent =
			admitted && input.content !== undefined && input.content !== prior.content
				? await this.#sendToServer(prior, {
						name: input.name ?? prior.name,
						content: input.content
					})
				: undefined
		const base = prior ? currentVersion(prior) : 0
		let refused = false
		let overtaken: PersistedArtifact | undefined
		const { outcome, artifact } = await mutateArtifact(id, (stored) => {
			// Read inside the mutator: hoisted out, it would weigh a stale copy against a fresh one.
			const held = this.artifacts.find((a) => a.id === id)
			const existing = furtherAlong(stored, held)
			if (
				!existing ||
				!isLive(existing) ||
				(opts?.sessionId !== undefined && existing.sessionId !== opts.sessionId)
			) {
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
			// Made on `prior`, and weighed by the server against it if it was asked at all, which
			// an edit leaving `prior`'s text as it is never is. Over a row that moved past `prior`,
			// it would land under a number the server never checked, the base of this browser's
			// next write. Two tabs' edits of an artifact the server has no copy of both land.
			if (prior && sent !== 'unshared' && currentVersion(existing) > base) {
				overtaken = existing
				return undefined
			}
			const edit = reviseInto(existing, input)
			edit.artifact.workspaceId ??= this.#home()
			return edit
		})
		if (refused) return undefined
		if (overtaken) this.#overtaken(overtaken, sent === 'saved' ? base + 1 : undefined)
		if (artifact?.role === 'plan' && outcome !== 'saved') throw new ArtifactPersistenceError()
		if (artifact) {
			this.#reflect(artifact)
			if (sent === 'behind') this.#bringServerUpToDate(artifact)
		}
		return artifact
	}

	/** An edit not stored because another tab stored a later version than the one it was made
	 * on: this list takes that version. `savedAs` is the version the server took the edit as,
	 * when it did. */
	#overtaken(by: PersistedArtifact, savedAs: number | undefined): never {
		if (isLive(by)) this.#reflect(by)
		throw new ArtifactStaleError(by, currentVersion(by), isLive(by), savedAs)
	}

	/**
	 * Hand the next version of an artifact to the server before this browser stores it: when the
	 * artifact is shared, people with access see it there. The server refuses a version based on
	 * an older one than the latest it knows, which its owner wrote from another browser since
	 * this one last did; that throws ArtifactStaleError, and a call that fails throws its error.
	 * Resolves `saved` when the server took the version, `unshared` when it holds no copy to take
	 * it into, and `behind` when its copy is older than this browser's, for the caller to bring up
	 * to date once the version is stored.
	 */
	async #sendToServer(
		prior: PersistedArtifact,
		next: { name: string; content: string }
	): Promise<'saved' | 'unshared' | 'behind'> {
		const workspace = prior.workspaceId ?? this.#home()
		if (!workspace) return 'unshared'
		const base = currentVersion(prior)
		// Not caught: a version stored here that the server never weighed is the base of this
		// browser's next write, which would then replace one written elsewhere in between.
		const result: AiArtifactWriteResult = await AiService.writeAiArtifact({
			workspace,
			artifactId: prior.id,
			requestBody: {
				base_version: base,
				version: base + 1,
				name: next.name,
				content: next.content,
				session_id: prior.sessionId
			}
		})
		if (result.outcome === 'saved') return 'saved'
		const latest = result.latest_version
		if (result.outcome !== 'stale' || latest === undefined) return 'unshared'
		if (latest < base) return 'behind'
		throw new ArtifactStaleError(
			prior,
			latest,
			await this.#takeServerCopy(prior, workspace, latest)
		)
	}

	/**
	 * Catch this browser up with version `latest`, the server's latest: from the store, where
	 * another tab may have put it, or else from the server's copy when that holds it. The server
	 * keeps only the version people see, an older one while its owner has one pinned. Resolves
	 * whether this browser now holds version `latest`.
	 */
	async #takeServerCopy(
		prior: PersistedArtifact,
		workspace: string,
		latest: number
	): Promise<boolean> {
		if (await this.#holds(prior.id, latest)) return true
		let copy: AiArtifactWithContent
		try {
			copy = await AiService.getAiArtifact({ workspace, artifactId: prior.id })
		} catch {
			return false
		}
		if (copy.shown_version !== latest) return false
		await this.#adopt(prior.id, copy)
		return this.#holds(prior.id, latest)
	}

	/** Whether the copy an edit is redone on, the one `get` reads, holds `version` or a later one,
	 * once this list has taken the stored row if that is further along. */
	async #holds(id: string, version: number): Promise<boolean> {
		const stored = await getArtifact(id).catch(() => undefined)
		const held = furtherAlong(
			stored,
			this.artifacts.find((a) => a.id === id)
		)
		if (!held || !isLive(held)) return false
		this.#reflect(held)
		return currentVersion(held) >= version
	}

	/**
	 * This browser's copy of an artifact, caught up with a newer version its owner wrote in
	 * another browser when `copy`, the server's, holds it: unless an older one is pinned there.
	 */
	async catchUp(local: PersistedArtifact, copy: AiArtifactWithContent): Promise<PersistedArtifact> {
		if (
			!isLive(local) ||
			copy.shown_version !== copy.latest_version ||
			copy.shown_version <= currentVersion(local)
		) {
			return local
		}
		return (await this.#adopt(local.id, copy)) ?? local
	}

	/**
	 * This browser's copy of an artifact once it holds `copy`'s version, the server's. When the
	 * store already holds that version or a later one, which another tab of this browser took,
	 * that copy is the one reflected: the in-memory list can be older than the store.
	 */
	async #adopt(id: string, copy: AiArtifactWithContent): Promise<PersistedArtifact | undefined> {
		const version = copy.shown_version
		let stored: PersistedArtifact | undefined
		const write = await mutateArtifact(id, (existing) => {
			stored = existing
			if (!existing || currentVersion(existing) >= version) return undefined
			const next: PersistedArtifact = {
				...existing,
				name: copy.name,
				content: copy.content,
				version,
				updatedAt: Date.now()
			}
			return {
				artifact: next,
				snapshots: [snapshotOf(next, version, 'Written in another browser')]
			}
		})
		const held = furtherAlong(
			write.artifact ?? stored,
			this.artifacts.find((a) => a.id === id)
		)
		if (held && isLive(held)) this.#reflect(held)
		return held
	}

	/** Give the server a version this browser stored without it, which its owner's writes from
	 * here are refused until it has. */
	#bringServerUpToDate(artifact: PersistedArtifact): void {
		const workspace = artifact.workspaceId ?? this.#home()
		if (!workspace) return
		publishArtifact(artifact, workspace).catch((err) =>
			console.error('Could not bring the server copy of the artifact up to date', err)
		)
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
		const prior = await this.get(id)
		const sent =
			prior !== undefined && prior.content !== revision.content
				? await this.#sendToServer(prior, revision)
				: undefined
		const base = prior ? currentVersion(prior) : 0
		let overtaken: PersistedArtifact | undefined
		const { outcome, artifact } = await mutateArtifact(id, (existing) => {
			// As in `update`. Without `prior` no plan was there to make this one on, and it
			// revises whatever another tab put in the slot meanwhile.
			if (prior && sent !== 'unshared' && existing && currentVersion(existing) > base) {
				overtaken = existing
				return undefined
			}
			if (existing) {
				const edit = reviseInto(isLive(existing) ? existing : revived(existing), revision)
				edit.artifact.workspaceId ??= this.#home()
				return edit
			}
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
				version: 1,
				workspaceId: this.#home()
			}
			return { artifact: created, snapshots: [snapshotOf(created, 1)] }
		})
		if (overtaken) this.#overtaken(overtaken, sent === 'saved' ? base + 1 : undefined)
		// A plan the database refused would let the user approve one that disappears on reload.
		if (!artifact || outcome !== 'saved') throw new ArtifactPersistenceError()
		this.#reflect(artifact)
		if (sent === 'behind') this.#bringServerUpToDate(artifact)
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

	/** Move an artifact to Recently deleted, where its owner can restore it for 30 days. Rejects
	 * when the server holds a copy it could not move there too. */
	async trash(id: string): Promise<void> {
		const workspace = (await this.get(id))?.workspaceId ?? this.#home()
		if (workspace) await trashArtifact(id, workspace)
		else await patchArtifact(id, { deletedAt: Date.now() })
		// Guard on presence: a no-op removal must not invalidate an in-flight load.
		const next = this.artifacts.filter((a) => a.id !== id)
		if (next.length !== this.artifacts.length) this.#applyWrite(next)
	}

	/** Bring an artifact back from Recently deleted, shared with whoever it was shared with. */
	async restore(id: string): Promise<void> {
		const workspace = (await getArtifact(id))?.workspaceId ?? this.#home()
		if (workspace) await restoreArtifact(id, workspace)
		else await patchArtifact(id, { deletedAt: undefined })
		const restored = await getArtifact(id)
		if (restored && isLive(restored)) this.#reflect(restored)
	}
}

function isLive(a: PersistedArtifact): boolean {
	return a.deletedAt === undefined
}

/**
 * A plan in Recently deleted, back for a new plan of its session to take its place: the plan id
 * is the session's, so the slot is still the deleted one's. Its versions carry on, which keeps
 * the server copy taking new ones, but not its approval. That copy stays in the trash, unshared
 * until shared again.
 */
function revived(plan: PersistedArtifact): PersistedArtifact {
	return { ...plan, deletedAt: undefined, approvedVersion: undefined }
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
