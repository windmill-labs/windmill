import { AiService } from '$lib/gen'
import type { ArtifactServer } from './artifactServer'

/** A call about one of the user's artifacts; undefined when the server holds no copy of it, which
 * is every artifact never shared. */
async function onServer<T>(call: () => Promise<T>): Promise<T | undefined> {
	try {
		return await call()
	} catch (err) {
		if ((err as { status?: number })?.status === 404) return undefined
		throw err
	}
}

export const httpArtifactServer: ArtifactServer = {
	publish: (workspace, requestBody) => AiService.publishAiArtifact({ workspace, requestBody }),
	write: (workspace, artifactId, requestBody) =>
		AiService.writeAiArtifact({ workspace, artifactId, requestBody }),
	show: (workspace, artifactId, requestBody) =>
		AiService.showAiArtifactVersion({ workspace, artifactId, requestBody }),
	get: (workspace, artifactId) =>
		onServer(() => AiService.getAiArtifact({ workspace, artifactId })),
	trash: async (workspace, artifactId) => {
		await onServer(() => AiService.trashAiArtifact({ workspace, artifactId }))
	},
	restore: async (workspace, artifactId) => {
		await onServer(() => AiService.restoreAiArtifact({ workspace, artifactId }))
	},
	delete: async (workspace, artifactId) => {
		await onServer(() => AiService.deleteAiArtifact({ workspace, artifactId }))
	}
}
