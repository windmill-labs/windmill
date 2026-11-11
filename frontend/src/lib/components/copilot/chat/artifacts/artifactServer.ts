import { hash } from '$lib/components/apps/editor/commonAppUtils'
import type {
	AiArtifact,
	AiArtifactWithContent,
	AiArtifactWriteResult,
	PublishAiArtifactData,
	ShowAiArtifactVersionData,
	WriteAiArtifactData
} from '$lib/gen'

/**
 * The server copy of the owner's artifacts, which people with access read. `httpArtifactServer`
 * is the API; the tests write through an in-memory one held to the same rules
 * (backend/tests/ai_artifacts_cases.json).
 */
export interface ArtifactServer {
	/** Make the artifact an item, or answer the copy already there (see publishAiArtifact). */
	publish(workspace: string, body: PublishAiArtifactData['requestBody']): Promise<AiArtifact>
	/** A new version, taken only when made on the latest one. */
	write(
		workspace: string,
		id: string,
		body: WriteAiArtifactData['requestBody']
	): Promise<AiArtifactWriteResult>
	/** Choose the version people with access see. */
	show(
		workspace: string,
		id: string,
		body: ShowAiArtifactVersionData['requestBody']
	): Promise<AiArtifactWriteResult>
	/** The copy, or undefined when the server holds none. */
	get(workspace: string, id: string): Promise<AiArtifactWithContent | undefined>
	/** Move the copy to the trash, or out of it; nothing to do when there is none. */
	trash(workspace: string, id: string): Promise<void>
	restore(workspace: string, id: string): Promise<void>
	/** Delete the copy for good, its owner's or, for an admin, anyone's; nothing to do when there
	 * is none. */
	delete(workspace: string, id: string): Promise<void>
}

/** What the server knows a version by: the SHA-256 of its name and text joined by a NUL, which
 * neither may contain. Two of the owner's browsers can each hold a different version under one
 * number. */
export function fingerprint(name: string, content: string): Promise<string> {
	return hash(`${name}\0${content}`)
}
