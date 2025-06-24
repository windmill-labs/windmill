import { type Writable, get } from 'svelte/store'
import type { FlowState } from './flows/flowState'
import { NEVER_TESTED_THIS_FAR } from './flows/models'
import { JobService, type Flow } from '$lib/gen'
import { dfs } from './flows/dfs'
import { getContext, setContext } from 'svelte'

export type stepState = {
	initial?: boolean
	loadingJobs?: boolean
}

const STEP_HISTORY_LOADER_CONTEXT_KEY = 'stepHistoryLoader'

export class StepHistoryLoader {
	#stepStates = $state<Record<string, stepState>>({})
	#flowJobInitial = $state<boolean | undefined>(undefined)
	#saveCb = $state<(() => void) | undefined>(undefined)
	#noInitial = $state<boolean>(false)

	get stepStates() {
		return this.#stepStates
	}

	set stepStates(value: Record<string, stepState>) {
		this.#stepStates = value
	}

	get flowJobInitial(): boolean | undefined {
		return this.#flowJobInitial
	}

	setFlowJobInitial(value: boolean | undefined) {
		if (this.#noInitial) {
			return
		}
		this.#flowJobInitial = value
		if (value === false) {
			// Set all step initials to false
			Object.keys(this.#stepStates).forEach((stepId) => {
				this.#stepStates[stepId].initial = false
			})
		}
		this.#saveCb?.()
	}

	constructor(
		stepStates: Record<string, stepState>,
		flowJobInitial?: boolean,
		saveCb?: () => void,
		noInitial?: boolean
	) {
		this.#stepStates = stepStates
		this.#flowJobInitial = flowJobInitial
		this.#saveCb = saveCb
		this.#noInitial = noInitial ?? false
	}

	resetInitial(stepId: string | undefined) {
		if (!stepId) {
			return
		}
		if (stepId in this.#stepStates && this.#stepStates[stepId]) {
			this.#stepStates[stepId].initial = false
		}

		/* // Check if all initial states are false
		const allInitialFalse = Object.values(this.#stepStates).every((state) => !state.initial)
		if (allInitialFalse) {
			this.#flowJobInitial = false
		} */

		this.#saveCb?.()
	}

	async loadIndividualStepsStates(
		flow: Flow,
		flowStateStore: Writable<FlowState>,
		workspaceId: string,
		initialPath: string,
		path: string
	): Promise<void> {
		// Collect all modules that need loading
		const modulesToLoad: any[] = []
		dfs(flow.value.modules, (module) => {
			const prev = get(flowStateStore)[module.id]?.previewResult
			if (!prev || prev === NEVER_TESTED_THIS_FAR) {
				modulesToLoad.push(module)
				// Initialize step state if it doesn't exist
				if (!this.#stepStates[module.id]) {
					this.#stepStates[module.id] = {
						initial: this.#noInitial ? false : undefined,
						loadingJobs: undefined
					}
				}
			}
		})

		// Load all modules in parallel and wait for completion
		const loadPromises = modulesToLoad.map(async (module) => {
			try {
				this.#stepStates[module.id].loadingJobs = true

				const scriptPath =
					`path` in module.value
						? module.value.path
						: (initialPath === '' ? path : initialPath) + '/' + module.id

				const previousJobId = await JobService.listJobs({
					workspace: workspaceId,
					scriptPathExact: scriptPath,
					jobKinds: ['preview', 'script', 'flowpreview', 'flow', 'flowscript'].join(','),
					page: 1,
					perPage: 1
				})

				if (previousJobId.length === 0) {
					this.#stepStates[module.id].loadingJobs = false
					return
				}

				const getJobResult = await JobService.getCompletedJobResultMaybe({
					workspace: workspaceId,
					id: previousJobId[0].id
				})

				if ('result' in getJobResult) {
					flowStateStore.update((state) => ({
						...state,
						[module.id]: {
							...(state[module.id] ?? {}),
							previewResult: getJobResult.result,
							previewJobId: previousJobId[0].id,
							previewWorkspaceId: previousJobId[0].workspace_id,
							previewSuccess: getJobResult.success
						}
					}))
					this.#stepStates[module.id].initial =
						this.#stepStates[module.id].initial !== undefined
							? this.#stepStates[module.id].initial
							: true
				}
			} catch (error) {
				console.error(`Failed to load history for module ${module.id}:`, error)
				this.#stepStates[module.id] = {
					initial: false,
					loadingJobs: false
				}
			} finally {
				// Ensure loading state is always cleaned up
				this.#stepStates[module.id].loadingJobs = false
			}
		})

		// Wait for all loading operations to complete
		await Promise.all(loadPromises)
	}

	private async getStatus(workspaceId: string, id: string) {
		const lastFlowJob = await JobService.getJob({
			workspace: workspaceId,
			id,
			noLogs: true
		})

		if (lastFlowJob.type !== 'CompletedJob' || !lastFlowJob.flow_status) {
			return
		}

		// Build a lookup of status modules by id for fast access during DFS
		const statusById: Record<string, any> = {}
		for (const m of lastFlowJob.flow_status.modules ?? []) {
			if (m?.id) {
				statusById[m.id] = m
			}
		}

		return statusById
	}

	/**
	 * Ensure a stepState entry exists for the given module id.
	 */
	private ensureStepState(moduleId: string) {
		if (!this.#stepStates[moduleId]) {
			this.#stepStates[moduleId] = {
				initial: this.#noInitial ? false : undefined,
				loadingJobs: undefined
			}
		}
	}

	/**
	 * Recursively traverse the flow definition and attach the latest execution
	 * result (if any) to the correct target module.
	 */
	private async applyLastResultRec(
		module: any,
		statusById: Record<string, any>,
		workspaceId: string,
		flowStateStore: Writable<FlowState>
	): Promise<void> {
		const statusModule = statusById[module.id]

		if (statusModule) {
			// Determine which job result to use for this module
			let jobId: string | undefined

			if (statusModule.job && statusModule.job !== '00000000-0000-0000-0000-000000000000') {
				jobId = statusModule.job
			}

			if (jobId) {
				try {
					const jobResult = await JobService.getCompletedJobResultMaybe({
						workspace: workspaceId,
						id: jobId
					})

					if ('result' in jobResult) {
						this.ensureStepState(module.id)

						flowStateStore.update((state) => ({
							...state,
							[module.id]: {
								...(state[module.id] ?? {}),
								previewResult: jobResult.result,
								previewJobId: jobId,
								previewWorkspaceId: workspaceId,
								previewSuccess: jobResult.success
							}
						}))

						this.#stepStates[module.id].initial =
							this.#stepStates[module.id].initial !== undefined
								? this.#stepStates[module.id].initial
								: true
					}
				} catch (error) {
					console.debug(`Could not load module job ${jobId}:`, error)
				}
			}
		}

		const isLoop = module.value?.type === 'forloopflow' || module.value?.type === 'whileloopflow'
		if (isLoop) {
			const statusById = await this.getStatus(
				workspaceId,
				statusModule.flow_jobs[statusModule.flow_jobs.length - 1] ?? ''
			)

			if (!statusById) {
				return
			}
			if (statusModule.flow_jobs && statusModule.flow_jobs.length > 0) {
				for (const child of module.value.modules) {
					await this.applyLastResultRec(child, statusById, workspaceId, flowStateStore)
				}
			}
		}

		// Recurse into branch children if present
		if (module.value?.branches) {
			for (const branch of module.value.branches) {
				if (branch.modules) {
					for (const child of branch.modules) {
						await this.applyLastResultRec(child, statusById, workspaceId, flowStateStore)
					}
				}
			}
		}

		// Recurse into the default path of a branchone module, if applicable
		if (module.value?.type === 'branchone' && module.value?.default) {
			for (const child of module.value.default) {
				await this.applyLastResultRec(child, statusById, workspaceId, flowStateStore)
			}
		}
	}

	async loadLastFlowExecution(
		flow: Flow,
		flowStateStore: Writable<FlowState>,
		workspaceId: string,
		initialPath: string,
		path: string
	): Promise<void> {
		try {
			const flowPath = initialPath === '' ? path : initialPath

			// Fetch the most recent completed flow/flowpreview job
			const lastFlowJobs = await JobService.listJobs({
				workspace: workspaceId,
				scriptPathExact: flowPath,
				jobKinds: ['flow', 'flowpreview'].join(','),
				page: 1,
				perPage: 1
			})

			if (lastFlowJobs.length === 0) {
				return
			}

			const statusById = await this.getStatus(workspaceId, lastFlowJobs[0].id)
			if (!statusById) {
				return
			}

			// Walk through the flow definition and attach the results
			for (const rootModule of flow.value.modules ?? []) {
				await this.applyLastResultRec(rootModule, statusById, workspaceId, flowStateStore)
			}
		} catch (error) {
			console.error('Failed to load last flow execution:', error)
		}
	}
}

export function getStepHistoryLoaderContext() {
	return getContext<StepHistoryLoader>(STEP_HISTORY_LOADER_CONTEXT_KEY)
}

export function setStepHistoryLoaderContext(stepHistoryLoader: StepHistoryLoader) {
	setContext<StepHistoryLoader>(STEP_HISTORY_LOADER_CONTEXT_KEY, stepHistoryLoader)
}
