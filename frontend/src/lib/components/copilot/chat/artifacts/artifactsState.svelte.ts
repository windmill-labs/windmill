import { randomUUID } from '$lib/utils/uuid'
import {
	deleteArtifact,
	getArtifact,
	listArtifactsForSession,
	putArtifact,
	type ArtifactKind,
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

	/** Load (or reload) the given session's artifacts into the reactive list. */
	async setSession(sessionId: string | undefined): Promise<void> {
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
			updatedAt: now
		}
		await putArtifact(artifact)
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
		const existing = this.artifacts.find((a) => a.id === id) ?? (await getArtifact(id))
		if (!existing) return undefined
		if (opts?.sessionId !== undefined && existing.sessionId !== opts.sessionId) return undefined
		const updated: PersistedArtifact = {
			...existing,
			name: input.name ?? existing.name,
			content: input.content ?? existing.content,
			updatedAt: Date.now()
		}
		await putArtifact(updated)
		if (updated.sessionId === this.#sessionId) {
			this.#applyWrite(sortByUpdatedDesc(this.artifacts.map((a) => (a.id === id ? updated : a))))
		}
		return updated
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
