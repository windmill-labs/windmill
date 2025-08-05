<script lang="ts">
	import { JobService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import type { FlowData } from './FlowLogUtils'
	import { untrack } from 'svelte'

	interface Props {
		flowData: FlowData | null
		expandedRows: Set<string>
		workspaceId: string | undefined
		refreshLog: boolean
	}

	let { flowData = $bindable(), expandedRows, workspaceId, refreshLog }: Props = $props()

	// Track polling state for each job
	let pollingStates: Record<
		string,
		{
			iteration: number
			logOffset: number
			lastJobId: string
		}
	> = $state({})

	// Get list of job IDs that need log polling (expanded steps with jobIds)
	function getJobIdsToWatch(): string[] {
		if (!flowData) return []
		return getJobIdsToWatchRecursive(flowData, 'root')
	}

	// Recursive function to handle subflows
	function getJobIdsToWatchRecursive(flowData: FlowData, flowId: string): string[] {
		const jobIds: string[] = []

		// Add main flow job if expanded
		if (expandedRows.has(`flow-${flowId}`) && flowData.jobId) {
			jobIds.push(flowData.jobId)
		}

		// Add step jobs if they are expanded and have jobIds
		for (const step of flowData.steps) {
			if (expandedRows.has(step.stepId) && step.jobId) {
				jobIds.push(step.jobId)
			}

			// Handle subflows recursively
			if (step.subflows && step.subflows.length > 0 && expandedRows.has(step.stepId)) {
				if (step.type === 'forloopflow' || step.type === 'whileloopflow') {
					// For loops: only check the selected iteration
					const selectedIteration = step.selectedIteration ?? 0
					if (step.subflows[selectedIteration]) {
						const subflowJobIds = getJobIdsToWatchRecursive(
							step.subflows[selectedIteration],
							step.stepId
						)
						jobIds.push(...subflowJobIds)
					}
				} else if (step.type === 'branchall' || step.type === 'branchone') {
					// For branches: check all subflows
					for (const subflow of step.subflows) {
						const subflowJobIds = getJobIdsToWatchRecursive(subflow, subflow.jobId)
						jobIds.push(...subflowJobIds)
					}
				} else if (step.type === 'flow') {
					// For flow steps: check the single subflow
					if (step.subflows[0]) {
						const subflowJobIds = getJobIdsToWatchRecursive(step.subflows[0], step.stepId)
						jobIds.push(...subflowJobIds)
					}
				}
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

	// Reset polling state when job changes
	function resetPollingState(jobId: string) {
		const state = pollingStates[jobId]
		if (state && state.lastJobId !== jobId) {
			pollingStates[jobId] = {
				iteration: 0,
				logOffset: 0,
				lastJobId: jobId
			}
		}
	}

	// Poll logs for a specific job
	async function pollLogsForJob(jobId: string) {
		if (!jobId || !flowData) return

		initPollingState(jobId)
		resetPollingState(jobId)

		const state = pollingStates[jobId]!
		state.iteration += 1

		try {
			const getUpdate = await JobService.getJobUpdates({
				workspace: workspaceId ?? $workspaceStore!,
				id: jobId,
				running: refreshLog,
				logOffset: state.logOffset === 0 ? getCurrentLogsLength(jobId) : state.logOffset
			})

			// Update logs in flowData
			updateLogsInFlowData(jobId, getUpdate.new_logs ?? '')

			// Update polling state
			state.logOffset = getUpdate.log_offset ?? 0
		} catch (error) {
			console.error('Failed to poll logs for job:', jobId, error)
		}

		// Schedule next poll if still refreshing
		if (refreshLog) {
			const iteration = state.iteration
			setTimeout(
				() => {
					if (refreshLog && getJobIdsToWatch().includes(jobId)) {
						pollLogsForJob(jobId)
					}
				},
				iteration < 10 ? 1000 : iteration < 20 ? 2000 : 5000
			)
		}
	}

	// Get current logs length for a job
	function getCurrentLogsLength(jobId: string): number {
		if (!flowData) return 0
		return getCurrentLogsLengthRecursive(flowData, jobId)
	}

	// Recursive function to get logs length in subflows
	function getCurrentLogsLengthRecursive(flowData: FlowData, jobId: string): number {
		// Check main flow logs
		if (flowData.jobId === jobId) {
			return flowData.logs?.length || 0
		}

		// Check step logs
		for (const step of flowData.steps) {
			if (step.jobId === jobId) {
				return step.logs?.length || 0
			}

			// Check subflows recursively
			if (step.subflows && step.subflows.length > 0) {
				for (const subflow of step.subflows) {
					const length = getCurrentLogsLengthRecursive(subflow, jobId)
					if (length > 0) {
						return length
					}
				}
			}
		}

		return 0
	}

	// Update logs in flowData structure
	function updateLogsInFlowData(jobId: string, newLogs: string) {
		if (!flowData || !newLogs) return
		updateLogsInFlowDataRecursive(flowData, jobId, newLogs)
	}

	// Recursive function to update logs in subflows
	function updateLogsInFlowDataRecursive(
		flowData: FlowData,
		jobId: string,
		newLogs: string
	): boolean {
		// Update main flow logs
		if (flowData.jobId === jobId) {
			flowData.logs = (flowData.logs || '').concat(newLogs)
			return true
		}

		// Update step logs
		for (const step of flowData.steps) {
			if (step.jobId === jobId) {
				step.logs = (step.logs || '').concat(newLogs)
				return true
			}

			// Check subflows recursively
			if (step.subflows && step.subflows.length > 0) {
				for (const subflow of step.subflows) {
					if (updateLogsInFlowDataRecursive(subflow, jobId, newLogs)) {
						return true
					}
				}
			}
		}

		return false
	}

	// Start polling for jobs that need it
	function startPolling() {
		const jobIds = getJobIdsToWatch()

		// Stop polling for jobs that are no longer needed
		const currentJobIds = new Set(jobIds)
		for (const jobId in pollingStates) {
			if (!currentJobIds.has(jobId)) {
				delete pollingStates[jobId]
			}
		}

		// Start polling for new jobs
		for (const jobId of jobIds) {
			if (refreshLog) {
				pollLogsForJob(jobId)
			}
		}
	}

	// React to changes in expanded rows or refresh state
	$effect(() => {
		expandedRows
		refreshLog
		untrack(() => {
			if (flowData && refreshLog) {
				startPolling()
			}
		})
	})

	$inspect('dbg')
</script>
