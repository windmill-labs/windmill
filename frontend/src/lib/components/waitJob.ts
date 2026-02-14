import { JobService } from '$lib/gen'
import { workspaceStore } from '$lib/stores'
import { get } from 'svelte/store'

const ITERATIONS_BEFORE_SLOW_REFRESH = 10
const ITERATIONS_BEFORE_SUPER_SLOW_REFRESH = 100

export async function waitJob(id: string, forceWorkspace?: string) {
	const workspace = forceWorkspace || get(workspaceStore)

	if (!id) {
		return
	}

	if (!workspace) {
		throw new Error('Workspace not found')
	}

	let syncIteration: number = 0
	let errorIteration: number = 0
	let job: any

	return new Promise((resolve, reject) => {
		async function checkJob() {
			try {
				const maybeJob = await JobService.getCompletedJobResultMaybe({
					workspace: workspace!,
					id,
					getStarted: false
				})

				if (maybeJob.completed) {
					job = { ...maybeJob, id }

					if (!job.success && typeof job.result === 'object' && 'error' in job.result) {
						reject(job.result.error)
					} else {
						resolve(job.result)
					}

					return
				}
			} catch (err) {
				errorIteration += 1
				if (errorIteration === 5) {
					try {
						await cancelJob(id, workspace!)
						return
					} catch (err) {
						console.error(err)
					}
				}
			}

			syncIteration++

			let nextIteration = 50

			if (syncIteration > ITERATIONS_BEFORE_SLOW_REFRESH) {
				nextIteration = 500
			} else if (syncIteration > ITERATIONS_BEFORE_SUPER_SLOW_REFRESH) {
				nextIteration = 2000
			}

			setTimeout(checkJob, nextIteration)
		}

		job = undefined
		checkJob()
	})
}

async function cancelJob(id: string, workspace: string) {
	if (id) {
		try {
			await JobService.cancelQueuedJob({
				workspace,
				id,
				requestBody: {}
			})
		} catch (err) {
			console.error(err)
		}
	}
}
