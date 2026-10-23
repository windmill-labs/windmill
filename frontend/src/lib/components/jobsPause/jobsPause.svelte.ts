import { WorkspaceService, type JobsPaused } from '$lib/gen'

/** The job pause of the workspace in view, shared by the banner and the workspace settings. */
class JobsPauseState {
	pause = $state<JobsPaused>(null)
	workspace = $state<string | undefined>(undefined)

	async load(workspace: string) {
		if (this.workspace !== workspace) {
			this.workspace = workspace
			this.pause = null
		}
		try {
			const pause = await WorkspaceService.getJobsPaused({ workspace })
			// A reply for a workspace the user already left must not overwrite the current one.
			if (this.workspace === workspace) {
				this.pause = pause
			}
		} catch (e) {
			console.error('Could not read whether jobs are paused', e)
		}
	}

	set(workspace: string, pause: JobsPaused) {
		this.workspace = workspace
		this.pause = pause
	}
}

export const jobsPause = new JobsPauseState()
