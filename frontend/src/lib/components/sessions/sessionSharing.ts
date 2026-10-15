import type { DisplayMessage } from '$lib/components/copilot/chat/shared'
import type {
	ArtifactKind,
	PersistedArtifact
} from '$lib/components/copilot/chat/artifacts/artifactsDB'

/** The server's `MAX_SNAPSHOT_BYTES`. */
export const MAX_SNAPSHOT_BYTES = 10 * 1024 * 1024

export type SharedSessionArtifact = {
	id: string
	name: string
	kind: ArtifactKind
	content: string
	role?: 'plan'
}

export type SessionSnapshot = {
	messages: DisplayMessage[]
	artifacts: SharedSessionArtifact[]
}

export function sharedSessionUrl(workspaceId: string, shareId: string): string {
	return `${window.location.origin}/shared_sessions/${encodeURIComponent(
		workspaceId
	)}/${encodeURIComponent(shareId)}`
}

/** A row as it reads once the turn is over: the viewer can neither wait on nor answer it. */
function settled(message: DisplayMessage, keepImages: boolean): DisplayMessage {
	switch (message.role) {
		case 'tool':
			return {
				...message,
				isLoading: false,
				isQueued: false,
				isStreamingArguments: false,
				needsConfirmation: false,
				resultStream: undefined
			}
		case 'assistant':
			return { ...message, streaming: false }
		case 'user':
			return keepImages ? message : { ...message, images: undefined }
		default:
			return message
	}
}

/**
 * The snapshot a share carries. Images go first when the whole would not fit: the
 * transcript is what the link is for.
 */
export function buildSessionSnapshot(
	messages: DisplayMessage[],
	artifacts: PersistedArtifact[]
): SessionSnapshot {
	const sharedArtifacts: SharedSessionArtifact[] = artifacts.map((a) => ({
		id: a.id,
		name: a.name,
		kind: a.kind,
		content: a.content,
		role: a.role
	}))
	for (const keepImages of [true, false]) {
		const snapshot: SessionSnapshot = {
			messages: messages.map((m) => settled(m, keepImages)),
			artifacts: sharedArtifacts
		}
		if (new TextEncoder().encode(JSON.stringify(snapshot)).length <= MAX_SNAPSHOT_BYTES) {
			return snapshot
		}
	}
	throw new Error(
		`the session is larger than the ${MAX_SNAPSHOT_BYTES / 1024 / 1024} MB a share can hold`
	)
}

const ROLES = new Set(['user', 'assistant', 'tool', 'summary'])

function isRecord(value: unknown): value is Record<string, unknown> {
	return typeof value === 'object' && value !== null && !Array.isArray(value)
}

/**
 * A snapshot as the viewer may render it. It is whatever the author's browser sent, so rows the
 * chat components cannot take are dropped, and an image renders only from an inline data URL:
 * any other source would load a third-party resource on behalf of whoever opens the link.
 */
export function readSessionSnapshot(raw: unknown): SessionSnapshot {
	const snapshot = isRecord(raw) ? raw : {}
	const messages = (Array.isArray(snapshot.messages) ? snapshot.messages : [])
		.filter(
			(m): m is Record<string, unknown> =>
				isRecord(m) && typeof m.role === 'string' && ROLES.has(m.role)
		)
		.map((m) => {
			const message: Record<string, unknown> = {
				...m,
				content: typeof m.content === 'string' ? m.content : ''
			}
			if (message.role === 'user') {
				message.images = (Array.isArray(m.images) ? m.images : []).filter(
					(i) => isRecord(i) && typeof i.dataUrl === 'string' && i.dataUrl.startsWith('data:image/')
				)
			}
			return message as unknown as DisplayMessage
		})
	const artifacts = (Array.isArray(snapshot.artifacts) ? snapshot.artifacts : []).filter(
		(a): a is SharedSessionArtifact =>
			isRecord(a) &&
			typeof a.id === 'string' &&
			typeof a.name === 'string' &&
			typeof a.content === 'string' &&
			(a.kind === 'md' || a.kind === 'html')
	)
	return { messages, artifacts }
}
