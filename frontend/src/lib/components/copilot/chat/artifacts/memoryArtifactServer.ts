import type { AiArtifact, AiArtifactWithContent, AiArtifactWriteResult } from '$lib/gen'
import { fingerprint, type ArtifactServer } from './artifactServer'

type Copy = Omit<AiArtifact, 'latest_hash'> & { latest_hash: string; content: string }

/**
 * The server copy of one owner's artifacts, held in memory, for tests. Takes, refuses and numbers
 * versions as the API does, which backend/tests/ai_artifacts_cases.json holds both to.
 */
export class MemoryArtifactServer implements ArtifactServer {
	readonly #copies = new Map<string, Copy>()
	#clock = 0

	constructor(readonly owner = 'owner') {}

	async publish(
		workspace: string,
		body: Parameters<ArtifactServer['publish']>[1]
	): Promise<AiArtifact> {
		const key = `${workspace}/${body.artifact_id}`
		const existing = this.#copies.get(key)
		if (existing) {
			existing.session_id ??= body.session_id
			if (existing.deleted_at !== undefined) {
				existing.extra_perms = {}
				existing.deleted_at = undefined
			}
			return item(existing)
		}
		const name = body.name.trim()
		const now = this.#now()
		const copy: Copy = {
			artifact_id: body.artifact_id,
			path: this.#freePath(workspace, name),
			extra_perms: {},
			owner: this.owner,
			is_owner: true,
			session_id: body.session_id,
			name,
			kind: body.kind,
			role: body.role,
			shown_version: body.version,
			latest_version: body.version,
			latest_hash: await fingerprint(name, body.content),
			pinned: false,
			created_at: now,
			updated_at: now,
			content: body.content
		}
		this.#copies.set(key, copy)
		return item(copy)
	}

	async write(
		workspace: string,
		id: string,
		body: Parameters<ArtifactServer['write']>[2]
	): Promise<AiArtifactWriteResult> {
		const copy = this.#live(workspace, id)
		if (!copy) return { outcome: 'not_shared' }
		if (copy.latest_hash !== body.base_hash) return refused(copy)
		const name = body.name.trim()
		const version = Math.max(copy.latest_version + 1, body.version)
		copy.latest_version = version
		copy.latest_hash = await fingerprint(name, body.content)
		copy.session_id ??= body.session_id
		if (!copy.pinned) {
			copy.content = body.content
			copy.name = name
			copy.shown_version = version
			copy.updated_at = this.#now()
		}
		return { outcome: 'saved', latest_version: version, artifact: item(copy) }
	}

	async show(
		workspace: string,
		id: string,
		body: Parameters<ArtifactServer['show']>[2]
	): Promise<AiArtifactWriteResult> {
		const copy = this.#live(workspace, id)
		if (!copy) return { outcome: 'not_shared' }
		if (
			copy.latest_hash !== body.base_hash ||
			(body.pinned && body.version > copy.latest_version)
		) {
			return refused(copy)
		}
		copy.pinned = body.pinned
		copy.shown_version = body.pinned ? body.version : copy.latest_version
		copy.content = body.content
		copy.name = body.name.trim()
		copy.updated_at = this.#now()
		return { outcome: 'saved', latest_version: copy.latest_version, artifact: item(copy) }
	}

	async get(workspace: string, id: string): Promise<AiArtifactWithContent | undefined> {
		const copy = this.#copies.get(`${workspace}/${id}`)
		return copy && { ...item(copy), owner_is_member: true, content: copy.content }
	}

	async trash(workspace: string, id: string): Promise<void> {
		const copy = this.#copies.get(`${workspace}/${id}`)
		if (copy) copy.deleted_at = this.#now()
	}

	async restore(workspace: string, id: string): Promise<void> {
		const copy = this.#copies.get(`${workspace}/${id}`)
		if (copy) copy.deleted_at = undefined
	}

	async delete(workspace: string, id: string): Promise<void> {
		this.#copies.delete(`${workspace}/${id}`)
	}

	#live(workspace: string, id: string): Copy | undefined {
		const copy = this.#copies.get(`${workspace}/${id}`)
		return copy?.deleted_at === undefined ? copy : undefined
	}

	#freePath(workspace: string, name: string): string {
		const slug =
			name
				.toLowerCase()
				.replace(/[^a-z0-9]+/g, '_')
				.replace(/^_+|_+$/g, '') || 'artifact'
		const base = `u/${this.owner}/${slug}`
		const taken = new Set(
			[...this.#copies.entries()]
				.filter(([key]) => key.startsWith(`${workspace}/`))
				.map(([, c]) => c.path)
		)
		if (!taken.has(base)) return base
		let n = 2
		while (taken.has(`${base}_${n}`)) n++
		return `${base}_${n}`
	}

	// Distinct, increasing times, so a test can tell which write came last.
	#now(): string {
		return new Date(Date.UTC(2026, 0, 1) + ++this.#clock * 1000).toISOString()
	}
}

function item(copy: Copy): AiArtifact {
	const { content: _content, ...rest } = copy
	return { ...rest, extra_perms: { ...rest.extra_perms } }
}

function refused(copy: Copy): AiArtifactWriteResult {
	return { outcome: 'stale', latest_version: copy.latest_version, latest_hash: copy.latest_hash }
}
