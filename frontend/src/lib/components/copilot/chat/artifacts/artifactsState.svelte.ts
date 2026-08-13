import { randomUUID } from '$lib/utils/uuid'
import {
	currentVersion,
	deleteArtifact,
	getArtifact,
	getArtifactVersion,
	listArtifactVersions,
	listArtifactsForSession,
	mutateArtifact,
	putArtifactWithVersions,
	versionKey,
	type ArtifactKind,
	type ArtifactVersion,
	type PersistedArtifact
} from './artifactsDB'

export interface CreateArtifactInput {
	name: string
	content: string
	kind?: ArtifactKind
	chatId?: string
}

export interface UpdateArtifactInput {
	name?: string
	content?: string
	/** Recorded on the snapshot this update produces; ignored if content is unchanged. */
	note?: string
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

	async get(id: string): Promise<PersistedArtifact | undefined> {
		// In-memory first: a write whose persist silently failed (quota) is still readable here.
		return this.artifacts.find((a) => a.id === id) ?? (await getArtifact(id))
	}

	async listForSession(sessionId: string): Promise<PersistedArtifact[]> {
		if (sessionId === this.#sessionId) return [...this.artifacts]
		return sortByUpdatedDesc(await listArtifactsForSession(sessionId))
	}

	/** Persist a new artifact for `sessionId` and reflect it in the list if that session is loaded. */
	async create(sessionId: string, input: CreateArtifactInput): Promise<PersistedArtifact> {
		const now = Date.now()
		const artifact: PersistedArtifact = {
			id: randomUUID(),
			sessionId,
			chatId: input.chatId,
			kind: input.kind ?? 'md',
			name: input.name,
			content: input.content,
			createdAt: now,
			updatedAt: now,
			version: 1
		}
		await putArtifactWithVersions(artifact, [snapshotOf(artifact, 1)])
		if (sessionId === this.#sessionId) {
			this.#applyWrite(sortByUpdatedDesc([artifact, ...this.artifacts]))
		}
		return artifact
	}

	/**
	 * Merge changes into an existing artifact. Returns undefined if `id` is unknown, or if
	 * `opts.sessionId` is given and the artifact belongs to a different session.
	 */
	async update(
		id: string,
		input: UpdateArtifactInput,
		opts?: { sessionId?: string }
	): Promise<PersistedArtifact | undefined> {
		const updated = await mutateArtifact(id, (stored) => {
			// Read inside the mutator: hoisted out, it would weigh a stale copy against a fresh one.
			const existing = furtherAlong(
				stored,
				this.artifacts.find((a) => a.id === id)
			)
			if (!existing) return undefined
			if (opts?.sessionId !== undefined && existing.sessionId !== opts.sessionId) return undefined
			// Only a content change earns a version: a rename or an identical rewrite would
			// otherwise fill the picker with entries the user cannot tell apart.
			const contentChanged = input.content !== undefined && input.content !== existing.content
			const version = currentVersion(existing) + (contentChanged ? 1 : 0)
			const artifact: PersistedArtifact = {
				...existing,
				name: input.name ?? existing.name,
				content: input.content ?? existing.content,
				updatedAt: Date.now(),
				version
			}
			const snapshots: ArtifactVersion[] = []
			// An artifact written before history existed has no snapshot of its current content,
			// so capture one on *any* update, not just a content change: this write stamps
			// `version`, and nothing afterwards would recognise it as pre-history.
			if (existing.version === undefined) {
				snapshots.push(snapshotOf(existing, currentVersion(existing)))
			}
			if (contentChanged) {
				snapshots.push(snapshotOf(artifact, version, input.note))
			}
			return { artifact, snapshots }
		})
		if (!updated) return undefined
		if (updated.sessionId === this.#sessionId) {
			this.#applyWrite(sortByUpdatedDesc(this.artifacts.map((a) => (a.id === id ? updated : a))))
		}
		return updated
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
