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

	// Simple polling state per job - like FlowJobResult.svelte
	let pollingStates: Record<
		string,
		{
			iteration: number
			logOffset: number
			lastJobId: string
		}
	> = $state({})

	// Get expanded job IDs that need polling
	function getExpandedJobIds(): string[] {
		const jobIds: string[] = []
		for (const [moduleId, moduleState] of Object.entries($localModuleStates)) {
			const isExpanded = expandedRows[moduleId] ?? allExpanded
			if (isExpanded && moduleState.job_id) {
				jobIds.push(moduleState.job_id)
			}
		}
		return jobIds
	}

	// Simple job change detection and log fetching - like FlowJobResult.svelte
	async function handleJobPolling(jobId: string) {
		if (!jobId) return

		// Initialize state for new job
		if (!pollingStates[jobId]) {
			pollingStates[jobId] = {
				iteration: 0,
				logOffset: 0,
				lastJobId: jobId
			}
		}

		const state = pollingStates[jobId]
		state.iteration += 1

		try {
			const getUpdate = await JobService.getJobUpdates({
				workspace: workspaceId ?? $workspaceStore!,
				id: jobId,
				running: refreshLog,
				logOffset: state.logOffset === 0 ? getCurrentLogsLength(jobId) : state.logOffset
			})

			// Update logs
			if (getUpdate.new_logs) {
				updateJobLogs(jobId, getUpdate.new_logs)
			}

			// Update offset
			state.logOffset = getUpdate.log_offset ?? 0

			// Schedule next poll if needed
			if (refreshLog && getExpandedJobIds().includes(jobId)) {
				const delay = state.iteration < 10 ? 1000 : state.iteration < 20 ? 2000 : 5000
				setTimeout(() => handleJobPolling(jobId), delay)
			}
		} catch (error) {
			console.error('Failed to get logs for job:', jobId, error)
		}
	}

	// Get current log length for offset calculation
	function getCurrentLogsLength(jobId: string): number {
		for (const moduleState of Object.values($localModuleStates)) {
			if (moduleState.job_id === jobId && moduleState.logs) {
				return moduleState.logs.length
			}
		}
		return 0
	}

	// Update job logs in module states
	function updateJobLogs(jobId: string, newLogs: string) {
		for (const [moduleId, moduleState] of Object.entries($localModuleStates)) {
			if (moduleState.job_id === jobId) {
				$localModuleStates[moduleId].logs = (moduleState.logs || '') + newLogs
				localModuleStates.update((state) => ({ ...state }))
				break
			}
		}
	}

	// React to expansion changes only - no circular dependency on localModuleStates
	$effect(() => {
		expandedRows
		allExpanded
		refreshLog
		untrack(() => {
			// Start polling for all currently expanded jobs
			for (const jobId of getExpandedJobIds()) {
				if (!pollingStates[jobId]) {
					handleJobPolling(jobId)
				}
			}
		})
	})
</script>
