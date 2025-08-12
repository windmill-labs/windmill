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
		localModuleStates: Writable<Record<string, GraphModuleState>>
	}

	let { expandedRows, allExpanded, workspaceId, localModuleStates }: Props = $props()

	// Polling state per moduleId - like FlowJobResult.svelte but for multiple modules
	let pollingStates: Record<
		string,
		{
			iteration: number
			logOffset: number
			lastJobId: string | undefined
		}
	> = $state({})

	// Get expanded module IDs that need polling
	function getExpandedModuleIds(): string[] {
		const moduleIds: string[] = []
		for (const [moduleId, moduleState] of Object.entries($localModuleStates)) {
			const isExpanded = expandedRows[moduleId] ?? allExpanded
			if (isExpanded && moduleState.job_id) {
				moduleIds.push(moduleId)
			}
		}
		return moduleIds
	}

	// Handle job change detection and log fetching per module - like FlowJobResult.svelte
	async function handleModulePolling(moduleId: string) {
		const moduleState = $localModuleStates[moduleId]
		if (!moduleState?.job_id) return

		const jobId = moduleState.job_id

		// Check if job ID changed for this module
		if (!pollingStates[moduleId] || pollingStates[moduleId].lastJobId !== jobId) {
			// Reset state for new job
			pollingStates[moduleId] = {
				iteration: 0,
				logOffset: 0,
				lastJobId: jobId
			}
			// Clear existing logs when job changes
			$localModuleStates[moduleId].logs = ''
		}

		const state = pollingStates[moduleId]
		state.iteration += 1

		try {
			const isInProgress = moduleState.type === 'InProgress'
			const currentLogsLength = moduleState.logs?.length || 0
			const getUpdate = await JobService.getJobUpdates({
				workspace: workspaceId ?? $workspaceStore!,
				id: jobId,
				running: isInProgress,
				logOffset:
					state.logOffset === 0 ? (currentLogsLength ? currentLogsLength + 1 : 0) : state.logOffset
			})

			// Update logs for this module (like FlowJobResult)
			if (getUpdate.new_logs) {
				$localModuleStates[moduleId].logs = (moduleState.logs || '') + getUpdate.new_logs
			}

			// Update offset
			state.logOffset = getUpdate.log_offset ?? 0

			// Schedule next poll if module is still in progress and expanded (like FlowJobResult)
			if (isInProgress && expandedModuleIds.includes(moduleId)) {
				const delay = state.iteration < 10 ? 1000 : state.iteration < 20 ? 2000 : 5000
				setTimeout(() => handleModulePolling(moduleId), delay)
			}
		} catch (error) {
			console.error('Failed to get logs for module:', moduleId, 'job:', jobId, error)
		}
	}

	const expandedModuleIds = $derived.by(getExpandedModuleIds)

	// React to expansion and module state changes
	$effect(() => {
		expandedModuleIds
		untrack(() => {
			// Handle polling for all currently expanded modules that are in progress
			for (const moduleId of expandedModuleIds) {
				const moduleState = $localModuleStates[moduleId]
				if (moduleState?.job_id) {
					// Check if we need to start/restart polling for this module
					if (
						!pollingStates[moduleId] ||
						pollingStates[moduleId].lastJobId !== moduleState.job_id
					) {
						handleModulePolling(moduleId)
					}
				}
			}
		})
	})
</script>
