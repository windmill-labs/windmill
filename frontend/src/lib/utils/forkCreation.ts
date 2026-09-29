import { WorkspaceService, type CreateWorkspaceFork } from '$lib/gen'

const POLL_INTERVAL_MS = 1500
const MAX_FAILED_POLLS = 10

/**
 * Create a fork and resolve once it exists, rejecting with the reason it could not be created.
 *
 * The copy runs on the server after the request returns: a large workspace takes longer to copy
 * than the timeout of many proxies in front of Windmill, which would otherwise cut the request.
 * `onStep` receives the part of the copy the server reports it is in.
 */
export async function createWorkspaceForkAndWait(
	parentWorkspace: string,
	fork: CreateWorkspaceFork,
	onStep?: (step: string) => void
): Promise<void> {
	try {
		await WorkspaceService.createWorkspaceFork({
			workspace: parentWorkspace,
			background: true,
			requestBody: fork
		})
	} catch (e) {
		// A retry of a request whose response was lost: wait for the creation it started.
		if (!String(e?.body ?? '').includes('is already being created')) throw e
	}
	// A poll that fails says nothing about the fork: it can be lost to the same proxy, or reach a
	// server without background forks (one of an older version during a rolling deploy), which
	// ignored the flag and created the fork before answering.
	let failedPolls = 0
	while (true) {
		let result: Awaited<ReturnType<typeof WorkspaceService.getForkCreationStatus>> | undefined
		try {
			result = await WorkspaceService.getForkCreationStatus({
				workspace: parentWorkspace,
				forkWorkspaceId: fork.id
			})
			failedPolls = 0
		} catch (e) {
			if (await forkExists(fork.id)) return
			if (++failedPolls >= MAX_FAILED_POLLS) throw e
		}
		if (result?.status === 'running' && result.step) onStep?.(result.step)
		if (result?.status === 'completed') return
		if (result?.status === 'failed') throw new Error(result.error ?? 'Unknown error')
		await new Promise((resolve) => setTimeout(resolve, POLL_INTERVAL_MS))
	}
}

async function forkExists(id: string): Promise<boolean> {
	try {
		return await WorkspaceService.existsWorkspace({ requestBody: { id } })
	} catch {
		return false
	}
}
