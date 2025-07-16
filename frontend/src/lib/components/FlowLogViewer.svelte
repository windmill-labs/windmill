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
	import FlowLogViewer from './FlowLogViewer.svelte'

	interface Props {
		innerModules: FlowStatusModule[]
		job: Job
		localModuleStates: Writable<Record<string, GraphModuleState>>
		workspaceId: string | undefined
		render: boolean
		prefix?: string
		level?: number
	}

	let {
		innerModules,
		job,
		localModuleStates,
		workspaceId,
		render,
		prefix,
		level = 0
	}: Props = $props()

	// State for tracking expanded rows - root steps expanded by default
	let expandedRows: Set<string> = $state(new Set())

	// Initialize expanded rows with root steps when level is 0 (root level)
	$effect(() => {
		if ((level || 0) === 0 && innerModules.length > 0) {
			const rootStepIds = innerModules
				.filter((module) => module.id)
				.map((module) => `start-${module.id}`)
			expandedRows = new Set(rootStepIds)
		}
	})

	// Cache for fetched logs per job ID
	let jobLogs: Map<string, string> = $state(new Map())

	// Cache for fetched subflow jobs
	let subflowJobs: Map<string, Job> = $state(new Map())

	// State for tracking selected iteration for forloop/whileloop steps
	let selectedIterations: Record<string, number> = $state({})

	// Reactive function to get selected iteration for a step
	function getSelectedIteration(stepId: string): number {
		return selectedIterations[stepId] ?? 0
	}

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

	// Fetch subflow job data
	async function fetchSubflowJob(jobId: string) {
		if (!jobId || subflowJobs.has(jobId)) return

		try {
			const jobData = await JobService.getJob({
				workspace: workspaceId ?? $workspaceStore ?? '',
				id: jobId,
				noLogs: true
			})

			subflowJobs.set(jobId, jobData)
			subflowJobs = new Map(subflowJobs)
		} catch (error) {
			console.error('Failed to fetch subflow job:', jobId, error)
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

	let logEntries = $derived(
		createLogEntries(innerModules, $localModuleStates, selectedIterations, subflowJobs)
	)

	// Effect to fetch logs when rows are expanded
	$effect(() => {
		for (const expandedRowId of expandedRows) {
			if (expandedRowId.startsWith('start-')) {
				const stepId = expandedRowId.replace('start-', '')
				const module = innerModules.find((m) => m.id === stepId)
				if (module?.job) {
					fetchJobLogs(module.job)
					// If this is a forloop/whileloop, fetch the first iteration by default
					if (module.flow_jobs && module.flow_jobs.length > 0) {
						const selectedIndex = getSelectedIteration(stepId)
						const jobId = module.flow_jobs?.[selectedIndex]
						if (jobId && !subflowJobs.has(jobId)) {
							fetchSubflowJob(jobId)
						}
					}
				}
			}
		}
	})

	// Effect to fetch subflow jobs when selected iterations change
	$effect(() => {
		// Access the selectedIterations object to ensure this effect runs when it changes
		selectedIterations

		for (const expandedRowId of expandedRows) {
			if (expandedRowId.startsWith('start-')) {
				const stepId = expandedRowId.replace('start-', '')
				const module = innerModules.find((m) => m.id === stepId)
				if (module?.flow_jobs && module.flow_jobs.length > 0) {
					const selectedIndex = getSelectedIteration(stepId)
					const jobId = module.flow_jobs?.[selectedIndex]
					if (jobId && !subflowJobs.has(jobId)) {
						fetchSubflowJob(jobId)
					}
				}
			}
		}
	})

	function createLogEntries(
		modules: FlowStatusModule[],
		moduleStates: Record<string, GraphModuleState>,
		selectedIterations: Record<string, number>,
		subflowJobs: Map<string, Job>
	): LogEntry[] {
		const entries: LogEntry[] = []

		modules.forEach((module, index) => {
			if (!module.id) return

			const stepNumber = index + 1
			const status = getStepStatus(module)
			const state = moduleStates[module.id]
			const summary = job.raw_flow?.modules?.[index]?.summary
			const isForloop = module.flow_jobs && module.flow_jobs.length > 0

			// For forloop steps, get the input for the selected iteration
			let iterationArgs = {}
			if (isForloop) {
				const selectedIndex = selectedIterations[module.id] ?? 0
				const selectedJobId = module.flow_jobs?.[selectedIndex]
				if (selectedJobId && subflowJobs.has(selectedJobId)) {
					iterationArgs = subflowJobs.get(selectedJobId)?.args || {}
				}
			}

			// Add start entry
			entries.push({
				id: `start-${module.id}`,
				type: 'start',
				stepId: module.id,
				stepNumber,
				jobId: isForloop ? '' : module.job || '', // No single job ID for forloop steps
				status,
				module,
				args: isForloop ? iterationArgs : state?.args || {}, // Show selected iteration input for forloop steps
				logs: isForloop ? '' : state?.logs || jobLogs.get(module.job || '') || '',
				summary
			})

			// Add end entry if job is completed
			if (status === 'success' || status === 'failure') {
				entries.push({
					id: `end-${module.id}`,
					type: 'end',
					stepId: module.id,
					stepNumber,
					jobId: isForloop ? '' : module.job || '', // No single job ID for forloop steps
					status,
					module,
					result: isForloop ? state?.flow_jobs_results || [] : state?.result, // Use concatenated results for forloop
					summary
				})
			}
		})

		return entries
	}
</script>

{#if render && logEntries.length > 0}
	<div class="bg-surface-secondary overflow-hidden">
		<table class="w-full font-mono text-xs">
			<tbody>
				{#each logEntries as entry (entry.id)}
					<tr class="border-b border-gray-200 dark:border-gray-700">
						<td class="p-1 w-full leading-tight">
							<div class="flex items-center justify-between">
								<div class="flex items-center gap-2">
									<button
										class="flex items-center text-xs text-tertiary hover:text-primary transition-colors"
										onclick={() => toggleExpanded(entry.id)}
									>
										{#if expandedRows.has(entry.id)}
											<ChevronDown size={12} />
										{:else}
											<ChevronRight size={12} />
										{/if}
									</button>

									<div class="flex items-center gap-2">
										{#if entry.type === 'start'}
											<span class="text-xs font-mono">
												Executing
												{#if entry.module.flow_jobs && entry.module.flow_jobs.length > 0}
													{#if job.raw_flow?.modules?.[entry.stepNumber - 1]?.value?.type === 'forloopflow'}
														forloop
													{:else if job.raw_flow?.modules?.[entry.stepNumber - 1]?.value?.type === 'whileloopflow'}
														whileloop
													{:else}
														step
													{/if}
												{:else}
													step
												{/if}
												<b>{entry.stepId}</b>
												{#if entry.summary}
													: {entry.summary}
												{/if}
											</span>
										{:else}
											<span class="text-xs font-mono">
												Step <b>{entry.stepId}</b>
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
										class="text-xs text-primary hover:underline font-mono"
										target="_blank"
										rel="noopener noreferrer"
									>
										{truncateRev(entry.jobId, 10)}
									</a>
								{/if}
							</div>

							{#if expandedRows.has(entry.id)}
								<div class="mt-1 pl-4 transition-all duration-200 ease-in-out">
									{#if entry.type === 'start'}
										<!-- Show input arguments -->
										{#if entry.args && Object.keys(entry.args).length > 0}
											<div class="mb-2">
												<h4 class="text-xs font-mono font-medium mb-1">Input:</h4>

												<ObjectViewer json={entry.args} pureViewer={true} />
											</div>
										{/if}

										<!-- Show iteration picker for forloop/whileloop or regular logs -->
										{#if entry.module.flow_jobs && entry.module.flow_jobs.length > 0}
											<!-- This is a forloop/whileloop, show iteration picker -->
											<div class="mb-2">
												<div class="flex items-center gap-2 mb-1">
													<h4 class="text-xs font-mono font-medium">
														{#if job.raw_flow?.modules?.[entry.stepNumber - 1]?.value?.type === 'forloopflow'}
															Forloop iterations
														{:else if job.raw_flow?.modules?.[entry.stepNumber - 1]?.value?.type === 'whileloopflow'}
															Whileloop iterations
														{:else}
															Subflow iterations
														{/if}
														({entry.module.flow_jobs?.length || 0}):
													</h4>
													<div class="w-48">
														{#each [''] as _}
															{@const iterationOptions =
																entry.module.flow_jobs?.map((jobId, index) => ({
																	label: `#${index + 1}: ${truncateRev(jobId, 8)}`,
																	value: index
																})) || []}
															{@const currentSelectedIndex = getSelectedIteration(entry.stepId)}
															<select
																value={currentSelectedIndex}
																onchange={(e) => {
																	const target = e.target as HTMLSelectElement
																	const selectedIndex = Number(target.value)
																	selectedIterations[entry.stepId] = selectedIndex
																	const jobId = entry.module.flow_jobs?.[selectedIndex]
																	if (jobId && !subflowJobs.has(jobId)) {
																		fetchSubflowJob(jobId)
																	}
																}}
																class="w-full px-1 text-xs border rounded bg-surface font-mono"
															>
																{#each iterationOptions as option}
																	<option value={option.value}>{option.label}</option>
																{/each}
															</select>
														{/each}
													</div>
												</div>

												{#each [''] as _}
													{@const selectedIndex = getSelectedIteration(entry.stepId)}
													{@const selectedJobId = entry.module.flow_jobs?.[selectedIndex]}

													{#if selectedJobId && subflowJobs.has(selectedJobId)}
														<FlowLogViewer
															innerModules={subflowJobs.get(selectedJobId)?.flow_status?.modules ||
																[]}
															job={subflowJobs.get(selectedJobId) || job}
															{localModuleStates}
															{workspaceId}
															{render}
															prefix={`${prefix || ''}${entry.stepId}.${selectedIndex + 1}.`}
															level={(level || 0) + 1}
														/>
													{:else if selectedJobId}
														<div class="text-xs text-tertiary font-mono">Loading iteration...</div>
													{/if}
												{/each}
											</div>
										{:else}
											<!-- Regular step, show logs -->
											{#if entry.logs}
												<div class="mb-2">
													<LogViewer
														content={entry.logs}
														jobId={entry.jobId}
														isLoading={entry.status === 'in_progress'}
														small={true}
														download={false}
														noAutoScroll={true}
														tag={undefined}
														noPadding
													/>
												</div>
											{:else if entry.jobId}
												<div class="mb-2">
													<div class="text-xs text-tertiary font-mono">
														{#if jobLogs.has(entry.jobId)}
															No logs available
														{:else}
															Loading logs...
														{/if}
													</div>
												</div>
											{/if}
										{/if}
									{:else if entry.type === 'end'}
										<!-- Show result -->
										{#if entry.result !== undefined}
											<div class="mb-2">
												<h4 class="text-xs font-mono font-medium mb-1">Result:</h4>

												<ObjectViewer json={entry.result} pureViewer={true} />
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
	<div class="p-2 text-tertiary text-xs italic font-mono"> No flow execution steps available </div>
{/if}

<style>
	.transition-all {
		transition: all 0.2s ease-in-out;
	}
</style>
