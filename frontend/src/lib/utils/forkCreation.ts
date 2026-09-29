import { WorkspaceService, type CreateWorkspaceFork } from '$lib/gen'

const POLL_INTERVAL_MS = 1500
const MAX_FAILED_POLLS = 5

/**
 * Create a fork and resolve once it exists, rejecting with the reason it could not be created.
 *
 * The copy runs on the server after the request returns: a large workspace takes longer to copy
 * than the timeout of many proxies in front of Windmill, which would otherwise cut the request.
 */
export async function createWorkspaceForkAndWait(
	parentWorkspace: string,
	fork: CreateWorkspaceFork
): Promise<void> {
	await WorkspaceService.createWorkspaceFork({
		workspace: parentWorkspace,
		background: true,
		requestBody: fork
	})
	// A poll lost to the same proxy says nothing about the fork, which is still being created.
	let failedPolls = 0
	while (true) {
		await new Promise((resolve) => setTimeout(resolve, POLL_INTERVAL_MS))
		let result: Awaited<ReturnType<typeof WorkspaceService.getForkCreationStatus>>
		try {
			result = await WorkspaceService.getForkCreationStatus({
				workspace: parentWorkspace,
				forkWorkspaceId: fork.id
			})
			failedPolls = 0
		} catch (e) {
			if (++failedPolls >= MAX_FAILED_POLLS) throw e
			continue
		}
		if (result.status === 'completed') return
		if (result.status === 'failed') throw new Error(result.error ?? 'Unknown error')
	}
}
