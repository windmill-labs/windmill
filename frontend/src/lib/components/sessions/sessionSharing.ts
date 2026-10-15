import type { DisplayMessage } from '$lib/components/copilot/chat/shared'
import type { ImageMediaType } from '$lib/components/copilot/chat/imageUtils'
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

function isInlineImage(url: unknown): url is string {
	return typeof url === 'string' && url.startsWith('data:image/')
}

function isRecord(value: unknown): value is Record<string, unknown> {
	return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function str(value: unknown): string | undefined {
	return typeof value === 'string' ? value : undefined
}

function records(value: unknown): Record<string, unknown>[] {
	return Array.isArray(value) ? value.filter(isRecord) : []
}

/**
 * A row rebuilt from the fields that only display. Everything else the live chat carries on a
 * row is an affordance acting as whoever has the page open (a run form, a question to answer,
 * a run to inspect, an item to open) or a pending state that mounts one, so it never survives:
 * the author controls every byte of the snapshot.
 */
function readMessage(m: Record<string, unknown>, index: number): DisplayMessage | undefined {
	const content = str(m.content) ?? ''
	switch (m.role) {
		case 'user':
			return {
				role: 'user',
				content,
				index,
				images: records(m.images)
					.filter((i) => isInlineImage(i.dataUrl))
					.map((i) => ({
						dataUrl: i.dataUrl as string,
						mediaType: i.mediaType as ImageMediaType,
						name: str(i.name)
					})),
				pastes: records(m.pastes)
					.filter((p) => typeof p.id === 'number' && typeof p.content === 'string')
					.map((p) => ({
						id: p.id as number,
						lines: typeof p.lines === 'number' ? p.lines : 0,
						content: p.content as string
					})),
				files: records(m.files)
					.filter((f) => typeof f.name === 'string' && typeof f.content === 'string')
					.map((f) => ({ name: f.name as string, content: f.content as string }))
			}
		case 'assistant':
			return {
				role: 'assistant',
				content,
				reasoning: str(m.reasoning),
				reasoningDurationMs:
					typeof m.reasoningDurationMs === 'number' ? m.reasoningDurationMs : undefined,
				createdAt: str(m.createdAt)
			}
		case 'tool':
			return {
				role: 'tool',
				tool_call_id: str(m.tool_call_id) ?? `shared-${index}`,
				content,
				toolName: str(m.toolName),
				parameters: m.parameters,
				result: m.result,
				logs: str(m.logs),
				error: str(m.error),
				imageUrl: isInlineImage(m.imageUrl) ? m.imageUrl : undefined,
				planArtifactId: str(m.planArtifactId),
				planVersion: typeof m.planVersion === 'number' ? m.planVersion : undefined,
				blockedByPlanMode: m.blockedByPlanMode === true,
				declinedByUser: m.declinedByUser === true
			}
		case 'summary':
			return { role: 'summary', content }
		default:
			return undefined
	}
}

/**
 * A snapshot as the viewer may render it: rows rebuilt by {@link readMessage}, and an attached
 * image only from an inline data URL, since any other source would load a third-party resource
 * on behalf of whoever opens the link. Images inside rendered text are left to the viewer
 * page's `img-src` policy.
 */
export function readSessionSnapshot(raw: unknown): SessionSnapshot {
	const snapshot = isRecord(raw) ? raw : {}
	const messages = records(snapshot.messages)
		.map(readMessage)
		.filter((m): m is DisplayMessage => m !== undefined)
	const artifacts = records(snapshot.artifacts)
		.filter(
			(a) =>
				typeof a.id === 'string' &&
				typeof a.name === 'string' &&
				typeof a.content === 'string' &&
				(a.kind === 'md' || a.kind === 'html')
		)
		.map((a) => ({
			id: a.id as string,
			name: a.name as string,
			kind: a.kind as ArtifactKind,
			content: a.content as string,
			role: a.role === 'plan' ? ('plan' as const) : undefined
		}))
	return { messages, artifacts }
}
