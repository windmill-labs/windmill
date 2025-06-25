import { type Writable, get } from 'svelte/store'
import type { FlowState } from './flows/flowState'
import { NEVER_TESTED_THIS_FAR } from './flows/models'
import { JobService, type Flow, type FlowModule } from '$lib/gen'
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

		// Check if all initial states are false
		const allInitialFalse = Object.values(this.#stepStates).every((state) => !state.initial)
		if (allInitialFalse) {
			this.#flowJobInitial = false
		}

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
		const modulesToLoad: FlowModule[] = []
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
}

export function getStepHistoryLoaderContext() {
	return getContext<StepHistoryLoader>(STEP_HISTORY_LOADER_CONTEXT_KEY)
}

export function setStepHistoryLoaderContext(stepHistoryLoader: StepHistoryLoader) {
	setContext<StepHistoryLoader>(STEP_HISTORY_LOADER_CONTEXT_KEY, stepHistoryLoader)
}
