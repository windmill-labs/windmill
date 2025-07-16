<script lang="ts">
	import { ChevronDown, ChevronRight } from 'lucide-svelte'
	import { base } from '$lib/base'
	import { workspaceStore } from '$lib/stores'
	import { truncateRev } from '$lib/utils'
	import ObjectViewer from './propertyPicker/ObjectViewer.svelte'
	import LogViewer from './LogViewer.svelte'
	import type { FlowStatusModule, Job } from '$lib/gen'
	import { JobService } from '$lib/gen'
	import type { GraphModuleState } from './graph'
	import type { Writable } from 'svelte/store'

	interface Props {
		innerModules: FlowStatusModule[]
		job: Job
		localModuleStates: Writable<Record<string, GraphModuleState>>
		workspaceId: string | undefined
		render: boolean
	}

	let { innerModules, job, localModuleStates, workspaceId, render }: Props = $props()

	// State for tracking expanded rows
	let expandedRows: Set<string> = $state(new Set())

	// Cache for fetched logs per job ID
	let jobLogs: Map<string, string> = $state(new Map())

	function toggleExpanded(stepId: string) {
		if (expandedRows.has(stepId)) {
			expandedRows.delete(stepId)
		} else {
			expandedRows.add(stepId)
		}
		expandedRows = new Set(expandedRows)
	}

	// Fetch logs for a specific job
	async function fetchJobLogs(jobId: string) {
		if (!jobId || jobLogs.has(jobId)) return

		try {
			const jobData = await JobService.getJob({
				workspace: workspaceId ?? $workspaceStore ?? '',
				id: jobId,
				noLogs: false
			})

			if (jobData.logs) {
				jobLogs.set(jobId, jobData.logs)
				jobLogs = new Map(jobLogs)
			}
		} catch (error) {
			console.error('Failed to fetch logs for job:', jobId, error)
		}
	}

	function getStepStatus(
		module: FlowStatusModule
	): 'success' | 'failure' | 'in_progress' | 'waiting' {
		if (module.type === 'Success') return 'success'
		if (module.type === 'Failure') return 'failure'
		if (module.type === 'InProgress') return 'in_progress'
		return 'waiting'
	}

	function getJobLink(jobId: string): string {
		return `${base}/run/${jobId}?workspace=${workspaceId ?? $workspaceStore}`
	}

	// Create log entries for each step
	interface LogEntry {
		id: string
		type: 'start' | 'end'
		stepId: string
		stepNumber: number
		jobId: string
		status: 'success' | 'failure' | 'in_progress' | 'waiting'
		module: FlowStatusModule
		args?: any
		result?: any
		logs?: string
		summary?: string
	}

	let logEntries = $derived(createLogEntries(innerModules, $localModuleStates))

	// Effect to fetch logs when rows are expanded
	$effect(() => {
		for (const expandedRowId of expandedRows) {
			if (expandedRowId.startsWith('start-')) {
				const stepId = expandedRowId.replace('start-', '')
				const module = innerModules.find((m) => m.id === stepId)
				if (module?.job) {
					fetchJobLogs(module.job)
				}
			}
		}
	})

	function createLogEntries(
		modules: FlowStatusModule[],
		moduleStates: Record<string, GraphModuleState>
	): LogEntry[] {
		const entries: LogEntry[] = []

		modules.forEach((module, index) => {
			if (!module.id) return

			const stepNumber = index + 1
			const status = getStepStatus(module)
			const state = moduleStates[module.id]
			const summary = job.raw_flow?.modules?.[index]?.summary

			// Add start entry
			entries.push({
				id: `start-${module.id}`,
				type: 'start',
				stepId: module.id,
				stepNumber,
				jobId: module.job || '',
				status,
				module,
				args: state?.args || {},
				logs: state?.logs || jobLogs.get(module.job || '') || '',
				summary
			})

			// Add end entry if job is completed
			if (status === 'success' || status === 'failure') {
				entries.push({
					id: `end-${module.id}`,
					type: 'end',
					stepId: module.id,
					stepNumber,
					jobId: module.job || '',
					status,
					module,
					result: state?.result,
					summary
				})
			}
		})

		return entries
	}
</script>

<div class="w-full">
	<h3 class="text-md leading-6 font-bold text-primary border-b mb-4 py-2"> Flow Execution Log </h3>

	{#if render && logEntries.length > 0}
		<div class="bg-surface border rounded-md overflow-hidden">
			<table class="w-full">
				<tbody>
					{#each logEntries as entry (entry.id)}
						<tr class="border-b border-gray-200 dark:border-gray-700">
							<td class="p-3 w-full">
								<div class="flex items-center justify-between">
									<div class="flex items-center gap-2">
										<button
											class="flex items-center text-sm text-tertiary hover:text-primary transition-colors"
											onclick={() => toggleExpanded(entry.id)}
										>
											{#if expandedRows.has(entry.id)}
												<ChevronDown size={16} />
											{:else}
												<ChevronRight size={16} />
											{/if}
										</button>

										<div class="flex items-center gap-2">
											{#if entry.type === 'start'}
												<span class="text-sm font-medium">
													Starting step {entry.stepNumber}
													{#if entry.summary}
														: {entry.summary}
													{/if}
												</span>
												<span class="text-xs text-tertiary">
													({entry.stepId})
												</span>
											{:else}
												<span class="text-sm font-medium">
													Step {entry.stepNumber}
													{#if entry.status === 'success'}
														<span class="text-green-600">executed with success</span>
													{:else if entry.status === 'failure'}
														<span class="text-red-600">failed</span>
													{/if}
												</span>
											{/if}
										</div>
									</div>

									{#if entry.jobId}
										<a
											href={getJobLink(entry.jobId)}
											class="text-xs text-primary hover:underline"
											target="_blank"
											rel="noopener noreferrer"
										>
											{truncateRev(entry.jobId, 10)}
										</a>
									{/if}
								</div>

								{#if expandedRows.has(entry.id)}
									<div class="mt-3 pl-6 transition-all duration-200 ease-in-out">
										{#if entry.type === 'start'}
											<!-- Show input arguments -->
											{#if entry.args && Object.keys(entry.args).length > 0}
												<div class="mb-4">
													<h4 class="text-sm font-medium mb-2">Input:</h4>
													<div class="bg-surface-secondary rounded p-2">
														<ObjectViewer json={entry.args} pureViewer={true} />
													</div>
												</div>
											{/if}

											<!-- Show logs -->
											{#if entry.logs}
												<div class="mb-4">
													<h4 class="text-sm font-medium mb-2">Logs:</h4>
													<div class="bg-surface-secondary rounded">
														<LogViewer
															content={entry.logs}
															jobId={entry.jobId}
															isLoading={entry.status === 'in_progress'}
															small={true}
															download={false}
															noAutoScroll={true}
															tag={undefined}
														/>
													</div>
												</div>
											{:else if entry.jobId}
												<div class="mb-4">
													<h4 class="text-sm font-medium mb-2">Logs:</h4>
													<div class="bg-surface-secondary rounded p-2 text-sm text-tertiary">
														{#if jobLogs.has(entry.jobId)}
															No logs available
														{:else}
															Loading logs...
														{/if}
													</div>
												</div>
											{/if}
										{:else if entry.type === 'end'}
											<!-- Show result -->
											{#if entry.result !== undefined}
												<div class="mb-4">
													<h4 class="text-sm font-medium mb-2">Result:</h4>
													<div class="bg-surface-secondary rounded p-2">
														<ObjectViewer json={entry.result} pureViewer={true} />
													</div>
												</div>
											{/if}
										{/if}
									</div>
								{/if}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{:else}
		<div class="p-4 text-tertiary text-sm italic"> No flow execution steps available </div>
	{/if}
</div>

<style>
	.transition-all {
		transition: all 0.2s ease-in-out;
	}
</style>
