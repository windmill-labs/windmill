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
	type ArtifactVersion,
	type PersistedArtifact
} from './artifactsDB'
import {
	ArtifactWriter,
	snapshotOf,
	type ArtifactChange,
	type CreateArtifactInput,
	type WriteResult,
	type WriteScope
} from './artifactWriter'

export type { ArtifactChange, CreateArtifactInput, WriteResult }

/**
 * Reactive view of the active session's artifacts, owned by AIChatManager (like
 * AttachedFilesStore). The consumer drives which session is loaded via setSession(); writes go
 * through an ArtifactWriter, and land in the list once it answers.
 */
export class SessionArtifactsStore {
	artifacts = $state<PersistedArtifact[]>([])
	loading = $state(false)

	#sessionId: string | undefined
	// A later load always wins, even if an earlier DB read resolves after it.
	#seq = 0
	#writer = new ArtifactWriter()
	/** The version of each artifact a chat last read or wrote, by `chatId/artifactId`: what the
	 * chat's next change to it is made on. */
	#seen = new Map<string, number>()

	/** Load the given session's artifacts into the reactive list, if it changed. */
	async setSession(sessionId: string | undefined): Promise<void> {
		if (sessionId === this.#sessionId) return
		this.#sessionId = sessionId
		await this.#load()
	}

	/** Re-read the loaded session's artifacts from the store, for records another tab wrote after
	 *  this one loaded. */
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

	// Insert-or-replace: a write can be the first this session hears of an artifact another tab
	// created.
	#reflect(artifact: PersistedArtifact): void {
		if (artifact.sessionId !== this.#sessionId) return
		const rest = this.artifacts.filter((a) => a.id !== artifact.id)
		this.#applyWrite(sortByUpdatedDesc([artifact, ...rest]))
	}

	async get(id: string): Promise<PersistedArtifact | undefined> {
		// In-memory first: a read-only view holds artifacts this browser never stored.
		return this.artifacts.find((a) => a.id === id) ?? (await getArtifact(id))
	}

	async listForSession(sessionId: string): Promise<PersistedArtifact[]> {
		if (sessionId === this.#sessionId) return [...this.artifacts]
		return sortByUpdatedDesc(await listArtifactsForSession(sessionId))
	}

	/** Record that `chatId` read version `version` of `artifact`, by default its latest: its next
	 * change is made on it. */
	markRead(
		chatId: string | undefined,
		artifact: PersistedArtifact,
		version = currentVersion(artifact)
	): void {
		if (chatId) this.#seen.set(`${chatId}/${artifact.id}`, version)
	}

	/** The latest version of the session's plan when it is past the one `chatId` last read or
	 * wrote, so a proposal made now would be refused as stale. */
	async stalePlanVersion(
		sessionId: string,
		chatId: string | undefined
	): Promise<number | undefined> {
		const row = await getArtifact(planArtifactId(sessionId))
		if (!row) return undefined
		const latest = currentVersion(row)
		const madeOn = this.#madeOn(chatId, row.id) ?? 0
		if (madeOn === latest) return undefined
		// As a stale proposal is settled: the list shows the latest, and the chat stays on what it
		// saw, or the newer row it now shows would pass for the one it read.
		this.#reflect(row)
		if (chatId) this.#seen.set(`${chatId}/${row.id}`, madeOn)
		return latest
	}

	/** The version a chat's next change to an artifact is made on: the one it last read or wrote,
	 * or else the one this tab shows. */
	#madeOn(chatId: string | undefined, id: string): number | undefined {
		const seen = chatId === undefined ? undefined : this.#seen.get(`${chatId}/${id}`)
		const shown = this.artifacts.find((a) => a.id === id)
		return seen ?? (shown ? currentVersion(shown) : undefined)
	}

	/** What a write did, reflected in the list, and remembered as what `chatId` holds: the version
	 * it stored, or for a change refused as made on an older one, that one, which keeps the chat's
	 * next change refused until it reads the latest instead of passing for one made on it. */
	#settle(result: WriteResult, chatId: string | undefined, madeOn?: number): WriteResult {
		const holds = (id: string, version: number | undefined) => {
			if (chatId && version !== undefined) this.#seen.set(`${chatId}/${id}`, version)
		}
		if (result.outcome === 'saved' || result.outcome === 'unchanged') {
			this.#reflect(result.artifact)
			const version = result.outcome === 'saved' ? result.version : currentVersion(result.artifact)
			holds(result.artifact.id, version)
		} else if (result.outcome === 'stale') {
			this.#reflect(result.artifact)
			holds(result.artifact.id, madeOn)
		}
		return result
	}

	/**
	 * A new artifact for `sessionId`. `scope.canWritePlan` is asked at the write rather than
	 * sampled by the caller: a posture entered while this write was waiting on the store still
	 * gets to decide it.
	 */
	async create(
		sessionId: string,
		input: CreateArtifactInput,
		scope?: Pick<WriteScope, 'canWritePlan'>
	): Promise<WriteResult> {
		return this.#settle(await this.#writer.create(sessionId, input, scope), input.chatId)
	}

	/** Store a change to an artifact, made on `opts.madeOn`, or else on the version `opts.chatId`
	 * last read or wrote. */
	async edit(
		id: string,
		change: ArtifactChange,
		opts: WriteScope & { chatId?: string; madeOn?: number } = {}
	): Promise<WriteResult> {
		const madeOn = opts.madeOn ?? this.#madeOn(opts.chatId, id)
		return this.#settle(await this.#writer.edit(id, change, madeOn, opts), opts.chatId, madeOn)
	}

	/** Put a proposal into the session's one plan document, made on the version of it `chatId`
	 * last read or wrote; a chat that saw none proposes the first. */
	async proposePlan(
		sessionId: string,
		revision: { name: string; content: string; note: string },
		chatId: string | undefined
	): Promise<WriteResult> {
		const madeOn = this.#madeOn(chatId, planArtifactId(sessionId)) ?? 0
		const result = await this.#writer.proposePlan(sessionId, revision, chatId, madeOn)
		return this.#settle(result, chatId, madeOn)
	}

	/**
	 * Stamp the version the user agreed to, and nothing else.
	 *
	 * Not `edit`: that rebuilds the row, so an approval computed while another tab was
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
				return { edit: { artifact: { ...existing, approvedVersion: version }, snapshots: [] } }
			},
			{ readVersion: version }
		)
		// An approval the store lost never happened, and showing the `plan` pill over it would put
		// the user's name on it anyway.
		if (!artifact || outcome !== 'saved') return false
		this.#reflect(artifact)
		return true
	}

	/**
	 * Every snapshot of an artifact, newest first. Empty if `id` is unknown, or if
	 * `opts.sessionId` is given and the artifact belongs to a different session — snapshots
	 * carry the document's full text, so this scopes like edit() rather than trusting
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
