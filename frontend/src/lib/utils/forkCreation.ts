import { WorkspaceService, type CreateWorkspaceFork } from '$lib/gen'

const POLL_INTERVAL_MS = 1500
// How long polls may keep failing, which a rolling deploy's replicas without the status route cause.
const MAX_FAILING_POLLS_MS = 3 * 60 * 1000

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
	const response = await WorkspaceService.createWorkspaceFork({
		workspace: parentWorkspace,
		background: true,
		requestBody: fork
	})
	// A server without background forks ignores the flag and answers once the fork is created.
	if (response.startsWith('Created forked workspace')) return
	await waitForForkCreation(parentWorkspace, fork.id, onStep)
}

/** Wait for the creation of a fork started in the background by this user from `parentWorkspace`. */
export async function waitForForkCreation(
	parentWorkspace: string,
	forkId: string,
	onStep?: (step: string) => void
): Promise<void> {
	let failingSince: number | undefined
	while (true) {
		let result: Awaited<ReturnType<typeof WorkspaceService.getForkCreationStatus>> | undefined
		try {
			result = await WorkspaceService.getForkCreationStatus({
				workspace: parentWorkspace,
				forkWorkspaceId: forkId
			})
			failingSince = undefined
		} catch (e) {
			// A failed poll says nothing about the fork: it can be lost to the same proxy the fork
			// runs in the background to avoid, or reach a replica that has no status route yet.
			failingSince ??= Date.now()
			if (Date.now() - failingSince >= MAX_FAILING_POLLS_MS) throw e
		}
		if (result?.status === 'running' && result.step) onStep?.(result.step)
		if (result?.status === 'completed') return
		if (result?.status === 'failed') throw new Error(result.error ?? 'Unknown error')
		await new Promise((resolve) => setTimeout(resolve, POLL_INTERVAL_MS))
	}
}
