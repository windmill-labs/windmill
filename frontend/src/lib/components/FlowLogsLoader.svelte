<script lang="ts">
	import { JobService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import type { GraphModuleState } from './graph'
	import type { Writable } from 'svelte/store'
	import { untrack } from 'svelte'

	interface Props {
		expandedRows: Record<string, boolean>
		allExpanded: boolean
		workspaceId: string | undefined
		refreshLog: boolean
		localModuleStates: Writable<Record<string, GraphModuleState>>
	}

	let { expandedRows, allExpanded, workspaceId, refreshLog, localModuleStates }: Props = $props()

	// Track polling state for each job - similar to FlowJobResult.svelte
	let pollingStates: Record<
		string,
		{
			iteration: number
			logOffset: number
			lastJobId: string
		}
	> = $state({})

	// Track which jobs we've seen before to trigger initial fetch
	let watchedJobIds: Set<string> = $state(new Set())

	// Get list of job IDs that need log polling directly from localModuleStates
	function getJobIdsToWatch(): string[] {
		const jobIds: string[] = []

		// Iterate through localModuleStates to find expanded steps with jobIds
		for (const [moduleId, moduleState] of Object.entries($localModuleStates)) {
			// If not in record, use allExpanded state. Otherwise use the explicit state from record
			const isExpanded = expandedRows[moduleId] ?? allExpanded
			if (isExpanded && moduleState.job_id) {
				jobIds.push(moduleState.job_id)
			}
		}

		return jobIds
	}

	// Initialize polling state for a job if it doesn't exist
	function initPollingState(jobId: string) {
		if (!pollingStates[jobId]) {
			pollingStates[jobId] = {
				iteration: 0,
				logOffset: 0,
				lastJobId: jobId
			}
		}
	}

	// Similar to diffJobId() in FlowJobResult.svelte - handles new job detection
	async function diffJobId(jobId: string) {
		if (!watchedJobIds.has(jobId)) {
			watchedJobIds.add(jobId)
			watchedJobIds = new Set(watchedJobIds)

			// Reset state for new job
			if (pollingStates[jobId]) {
				pollingStates[jobId] = {
					iteration: 0,
					logOffset: 0,
					lastJobId: jobId
				}
			}

			// Always fetch logs for new jobs (same as FlowJobResult.svelte line 60)
			await getLogs(jobId)
		}
	}

	// Similar to getLogs() in FlowJobResult.svelte
	async function getLogs(jobId: string) {
		if (!jobId) return

		initPollingState(jobId)
		const state = pollingStates[jobId]!
		state.iteration += 1

		try {
			const getUpdate = await JobService.getJobUpdates({
				workspace: workspaceId ?? $workspaceStore!,
				id: jobId,
				running: refreshLog,
				logOffset: state.logOffset === 0 ? getCurrentLogsLength(jobId) : state.logOffset
			})

			// Update logs in localModuleStates
			updateLogsInModuleStates(jobId, getUpdate.new_logs ?? '')

			// Update polling state
			state.logOffset = getUpdate.log_offset ?? 0
		} catch (error) {
			console.error('Failed to get logs for job:', jobId, error)
		}

		// Schedule next poll only if refreshLog is true (same as FlowJobResult.svelte lines 76-85)
		if (refreshLog) {
			const iteration = state.iteration
			setTimeout(
				() => {
					if (refreshLog && getJobIdsToWatch().includes(jobId)) {
						getLogs(jobId)
					}
				},
				iteration < 10 ? 1000 : iteration < 20 ? 2000 : 5000
			)
		}
	}

	// Get current logs length for a job from localModuleStates
	function getCurrentLogsLength(jobId: string): number {
		// Find the moduleId that has this jobId
		for (const [_, moduleState] of Object.entries($localModuleStates)) {
			if (moduleState.job_id === jobId && moduleState.logs) {
				return moduleState.logs.length
			}
		}
		return 0
	}

	// Update logs in localModuleStates
	function updateLogsInModuleStates(jobId: string, newLogs: string) {
		if (!newLogs) return

		// Find the moduleId that has this jobId and update its logs
		for (const [moduleId, moduleState] of Object.entries($localModuleStates)) {
			if (moduleState.job_id === jobId) {
				// Update the logs in localModuleStates
				$localModuleStates[moduleId].logs = (moduleState.logs || '').concat(newLogs)

				// Trigger reactivity
				localModuleStates.update((state) => ({ ...state }))
				break
			}
		}
	}

	// Start polling for jobs that need it - follows FlowJobResult pattern
	function startPolling() {
		const jobIds = getJobIdsToWatch()

		// Clean up polling states for jobs that are no longer needed
		const currentJobIds = new Set(jobIds)
		for (const jobId in pollingStates) {
			if (!currentJobIds.has(jobId)) {
				delete pollingStates[jobId]
			}
		}

		// Clean up watched job IDs for jobs that are no longer expanded
		const newWatchedJobIds = new Set<string>()
		for (const jobId of watchedJobIds) {
			if (currentJobIds.has(jobId)) {
				newWatchedJobIds.add(jobId)
			}
		}
		watchedJobIds = newWatchedJobIds

		// Process each job ID - always fetch logs for new jobs, regardless of refreshLog
		for (const jobId of jobIds) {
			diffJobId(jobId)
		}
	}

	// React to changes in expanded rows, allExpanded, refresh state, or localModuleStates
	$effect(() => {
		expandedRows
		allExpanded
		refreshLog
		$localModuleStates
		untrack(() => {
			startPolling()
		})
	})
</script>
