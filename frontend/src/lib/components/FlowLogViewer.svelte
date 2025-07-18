<script lang="ts">
	import { ChevronDown, ChevronRight } from 'lucide-svelte'
	import { base } from '$lib/base'
	import { workspaceStore } from '$lib/stores'
	import { truncateRev } from '$lib/utils'
	import ObjectViewer from './propertyPicker/ObjectViewer.svelte'
	import LogViewer from './LogViewer.svelte'
	import FlowLogViewer from './FlowLogViewer.svelte'

	interface FlowData {
		jobId: string
		inputs: any
		result: any
		steps: StepData[]
	}

	interface StepData {
		stepId: string
		stepNumber: number
		summary?: string
		inputs: any
		result?: any
		jobId?: string
		logs?: string
		status: 'success' | 'failure' | 'in_progress' | 'waiting'
		subflows?: FlowData[]
		iterations?: FlowData[]
		selectedIteration?: number
	}

	interface Props {
		flowData: FlowData
		expandedRows: Set<string>
		selectedIterations: Record<string, number>
		toggleExpanded: (id: string) => void
		updateSelectedIteration: (stepId: string, iteration: number) => void
		workspaceId: string | undefined
		render: boolean
		level?: number
	}

	let {
		flowData,
		expandedRows,
		selectedIterations,
		toggleExpanded,
		updateSelectedIteration,
		workspaceId,
		render,
		level = 0
	}: Props = $props()

	function getJobLink(jobId: string): string {
		return `${base}/run/${jobId}?workspace=${workspaceId ?? $workspaceStore}`
	}

	function getSelectedIteration(stepId: string): number {
		return selectedIterations[stepId] ?? 0
	}

	function handleIterationChange(stepId: string, newIteration: number) {
		updateSelectedIteration(stepId, newIteration)
	}

	// Create log entries for display
	interface LogEntry {
		id: string
		type: 'start' | 'end'
		stepId: string
		stepNumber: number
		jobId: string
		status: 'success' | 'failure' | 'in_progress' | 'waiting'
		stepData: StepData
		args?: any
		result?: any
		logs?: string
		summary?: string
	}

	function createLogEntries(steps: StepData[]): LogEntry[] {
		const entries: LogEntry[] = []

		steps.forEach((step) => {
			// Add start entry
			entries.push({
				id: `start-${step.stepId}`,
				type: 'start',
				stepId: step.stepId,
				stepNumber: step.stepNumber,
				jobId: step.jobId || '',
				status: step.status,
				stepData: step,
				args: step.inputs,
				logs: step.logs || '',
				summary: step.summary
			})

			// Add end entry if step is completed
			if (step.status === 'success' || step.status === 'failure') {
				entries.push({
					id: `end-${step.stepId}`,
					type: 'end',
					stepId: step.stepId,
					stepNumber: step.stepNumber,
					jobId: step.jobId || '',
					status: step.status,
					stepData: step,
					result: step.result,
					summary: step.summary
				})
			}
		})

		return entries
	}

	let logEntries = $derived(createLogEntries(flowData.steps))
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
												{#if entry.stepData.iterations && entry.stepData.iterations.length > 0}
													{#if entry.stepData.iterations.length > 1}
														forloop
													{:else}
														whileloop
													{/if}
												{:else if entry.stepData.subflows && entry.stepData.subflows.length > 0}
													branch
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

										<!-- Show iteration/subflow content -->
										{#if entry.stepData.iterations && entry.stepData.iterations.length > 0}
											<!-- This is a forloop/whileloop, show iteration picker -->
											<div class="mb-2">
												<div class="flex items-center gap-2 mb-1">
													<h4 class="text-xs font-mono font-medium">
														Iterations ({entry.stepData.iterations.length}):
													</h4>
													<div class="w-48">
														<select
															value={getSelectedIteration(entry.stepId)}
															onchange={(e) => {
																const target = e.target as HTMLSelectElement
																handleIterationChange(entry.stepId, Number(target.value))
															}}
															class="w-full px-1 text-xs border rounded bg-surface font-mono"
														>
															{#each entry.stepData.iterations as iter, index}
																<option value={index}>
																	#{index + 1}: {truncateRev(iter.jobId, 8)}
																</option>
															{/each}
														</select>
													</div>
												</div>

												{#if entry.stepData.iterations[getSelectedIteration(entry.stepId)]}
													<FlowLogViewer
														flowData={entry.stepData.iterations[getSelectedIteration(entry.stepId)]}
														{expandedRows}
														{selectedIterations}
														{toggleExpanded}
														{updateSelectedIteration}
														{workspaceId}
														{render}
														level={level + 1}
													/>
												{/if}
											</div>
										{:else if entry.stepData.subflows && entry.stepData.subflows.length > 0}
											<!-- This is a branch, show all subflows -->
											<div class="mb-2">
												<h4 class="text-xs font-mono font-medium mb-1">
													Subflows ({entry.stepData.subflows.length}):
												</h4>
												{#each entry.stepData.subflows as subflow, index}
													<div class="mb-2 border-l-2 border-gray-300 pl-2">
														<div class="text-xs font-mono font-medium mb-1">
															Branch #{index + 1}:
														</div>
														<FlowLogViewer
															flowData={subflow}
															{expandedRows}
															{selectedIterations}
															{toggleExpanded}
															{updateSelectedIteration}
															{workspaceId}
															{render}
															level={level + 1}
														/>
													</div>
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
													<div class="text-xs text-tertiary font-mono"> No logs available </div>
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
	<div class="p-2 text-tertiary text-xs italic font-mono">No flow execution steps available</div>
{/if}

<style>
	.transition-all {
		transition: all 0.2s ease-in-out;
	}
</style>
