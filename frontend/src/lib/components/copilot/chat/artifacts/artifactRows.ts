import type { AiArtifactListEntry } from '$lib/gen'
import { currentVersion, type ArtifactKind, type PersistedArtifact } from './artifactsDB'

/** One row of the artifacts page: an artifact in this browser, on the server, or both. */
export type ArtifactRow = {
	id: string
	name: string
	kind: ArtifactKind
	role?: 'plan'
	mine: boolean
	/** The owner's username, for someone else's artifact. */
	owner?: string
	/** False once the owner has left the workspace. */
	ownerIsMember: boolean
	/** The newest version: the one this browser holds for mine, the one shown for others'. */
	version: number
	updatedAt: number
	sessionId?: string
	/** The server copy's access list; undefined for an artifact never shared. */
	extraPerms?: Record<string, boolean>
	path?: string
	/** The version people with access see, when the owner pinned one. */
	pinnedVersion?: number
	deletedAt?: number
	/** The owner's copy, which only the browser that wrote it holds. */
	local?: PersistedArtifact
	/** The first line of the content matching the search. */
	matchLine?: string
}

/**
 * The page's rows: the user's own artifacts as this browser holds them, with the sharing state of
 * their server copy, then the ones only the server holds — mine written in another browser, and
 * other people's. `local` is the user's artifacts of this workspace, `server` the server's list.
 */
export function mergeArtifactRows(
	local: PersistedArtifact[],
	server: AiArtifactListEntry[]
): ArtifactRow[] {
	const byId = new Map(server.map((s) => [s.artifact_id, s]))
	const rows: ArtifactRow[] = local.map((a) => {
		const s = byId.get(a.id)
		const copy = s?.is_owner ? s : undefined
		return {
			id: a.id,
			name: a.name,
			kind: a.kind,
			role: a.role,
			mine: true,
			ownerIsMember: true,
			version: currentVersion(a),
			updatedAt: a.updatedAt,
			sessionId: a.sessionId,
			extraPerms: copy?.extra_perms,
			path: copy?.path,
			pinnedVersion: copy?.pinned ? copy.shown_version : undefined,
			deletedAt: a.deletedAt,
			local: a
		}
	})
	const held = new Set(local.map((a) => a.id))
	for (const s of server) {
		if (held.has(s.artifact_id)) continue
		rows.push({
			id: s.artifact_id,
			name: s.name,
			kind: s.kind,
			role: s.role,
			mine: s.is_owner,
			owner: s.is_owner ? undefined : s.owner,
			ownerIsMember: s.owner_is_member,
			version: s.is_owner ? s.latest_version : s.shown_version,
			updatedAt: Date.parse(s.updated_at),
			sessionId: s.session_id,
			extraPerms: s.extra_perms,
			path: s.path,
			pinnedVersion: s.pinned ? s.shown_version : undefined,
			deletedAt: s.deleted_at ? Date.parse(s.deleted_at) : undefined,
			matchLine: s.match_line
		})
	}
	return rows
}

/**
 * The first line of `content` holding `query`, case-insensitively, cut to a window around the
 * match when it is long, as the server's search (`match_line` in ai_artifacts.rs) reports it.
 */
export function matchLine(content: string, query: string, max = 160): string | undefined {
	const q = query.toLowerCase()
	const line = content
		.split('\n')
		.find((l) => l.toLowerCase().includes(q))
		?.trim()
	if (line === undefined) return undefined
	const chars = [...line]
	if (chars.length <= max) return line
	const lower = line.toLowerCase()
	const hit = [...lower.slice(0, lower.indexOf(q))].length
	const lead = Math.floor(Math.max(0, max - [...q].length) / 2)
	const start = Math.min(Math.max(0, hit - lead), chars.length - max)
	const cut = chars.slice(start, start + max).join('')
	return `${start > 0 ? '…' : ''}${cut}${start + max < chars.length ? '…' : ''}`
}
