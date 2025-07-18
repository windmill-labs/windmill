<script lang="ts">
	import { ChevronDown, ChevronRight } from 'lucide-svelte'
	import { base } from '$lib/base'
	import { workspaceStore } from '$lib/stores'
	import { truncateRev } from '$lib/utils'
	import ObjectViewer from './propertyPicker/ObjectViewer.svelte'
	import LogViewer from './LogViewer.svelte'
	import FlowLogViewer from './FlowLogViewer.svelte'
	import type { FlowData, StepData } from './FlowLogUtils'

	interface Props {
		flowData: FlowData
		expandedRows: Set<string>
		toggleExpanded: (id: string) => void
		updateSelectedIteration: (stepId: string, iteration: number) => void
		workspaceId: string | undefined
		render: boolean
		level?: number
	}

	let {
		flowData,
		expandedRows,
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
		return flowData.steps.find((step) => step.stepId === stepId)?.selectedIteration ?? 0
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
		stepType:
			| 'forloopflow'
			| 'whileloopflow'
			| 'branchall'
			| 'branchone'
			| 'script'
			| 'flow'
			| 'identity'
			| 'rawscript'
			| undefined
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
				summary: step.summary,
				stepType: step.type
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
					summary: step.summary,
					stepType: step.type
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
						<td class="py-2 leading-tight align-top">
							<button
								class="w-4 flex items-center justify-center text-xs text-tertiary hover:text-primary transition-colors"
								onclick={() => toggleExpanded(entry.id)}
							>
								{#if expandedRows.has(entry.id)}
									<ChevronDown size={8} />
								{:else}
									<ChevronRight size={8} />
								{/if}
							</button>
						</td>
						<td class="w-full leading-tight">
							<!-- svelte-ignore a11y_click_events_have_key_events -->
							<!-- svelte-ignore a11y_no_static_element_interactions -->
							<div
								class="py-1 flex items-center justify-between cursor-pointer"
								onclick={() => toggleExpanded(entry.id)}
							>
								<div class="flex items-center gap-2">
									{#if entry.type === 'start'}
										<span class="text-xs font-mono">
											Running
											<b>
												{#if entry.stepType === 'forloopflow'}
													forloop
												{:else if entry.stepType === 'whileloopflow'}
													whileloop
												{:else if entry.stepType === 'branchall'}
													branchall
												{:else if entry.stepType === 'branchone'}
													brancheone
												{:else}
													step
												{/if}
												{entry.stepId}
											</b>
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
										{#if entry.stepData.iterations && entry.stepData.iterations.length > 0}
											{#if entry.stepData.iterations[getSelectedIteration(entry.stepId)]}
												<span
													class="text-xs font-mono font-medium inline-flex items-center gap-1 w-full"
												>
													Running iteration&nbsp;
													<select
														value={getSelectedIteration(entry.stepId)}
														onchange={(e) => {
															const target = e.target as HTMLSelectElement
															handleIterationChange(entry.stepId, Number(target.value))
														}}
														class="inline-block !w-12 !p-0.5 !text-xs bg-surface-secondary font-mono"
													>
														{#each entry.stepData.iterations as _, index}
															<option value={index}>
																{index + 1}
															</option>
														{/each}
													</select>
													{`/${entry.stepData.iterations.length}`}
												</span>

												<div class="border-l">
													{#if entry.args && Object.keys(entry.args).length > 0}
														<div class="mb-2 ml-2">
															<h4 class="text-xs font-mono font-medium mb-1">Input:</h4>
															<ObjectViewer json={entry.args} pureViewer={true} />
														</div>
													{/if}
													<FlowLogViewer
														flowData={entry.stepData.iterations[getSelectedIteration(entry.stepId)]}
														{expandedRows}
														{toggleExpanded}
														{updateSelectedIteration}
														{workspaceId}
														{render}
														level={level + 1}
													/>
												</div>
											{/if}
										{:else if entry.stepData.subflows && entry.stepData.subflows.length > 0}
											<!-- This is a branch, show all subflows -->
											<div class="mb-2">
												<h4 class="text-xs font-mono font-medium mb-1">
													Subflows ({entry.stepData.subflows.length}):
												</h4>
												<!-- Show input arguments -->
												{#if entry.args && Object.keys(entry.args).length > 0}
													<div class="mb-2">
														<h4 class="text-xs font-mono font-medium mb-1">Input:</h4>
														<ObjectViewer json={entry.args} pureViewer={true} />
													</div>
												{/if}
												{#each entry.stepData.subflows as subflow, index}
													<div class="mb-2 border-l border-gray-300 pl-2">
														<div class="text-xs font-mono font-medium mb-1">
															Branch #{index + 1}:
														</div>
														<FlowLogViewer
															flowData={subflow}
															{expandedRows}
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
											<!-- Show input arguments -->
											{#if entry.args && Object.keys(entry.args).length > 0}
												<div class="mb-2">
													<h4 class="text-xs font-mono font-medium mb-1">Input:</h4>
													<ObjectViewer json={entry.args} pureViewer={true} />
												</div>
											{/if}
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
											<div>
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
