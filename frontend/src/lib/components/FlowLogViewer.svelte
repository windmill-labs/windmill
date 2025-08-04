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
		flowId: string
	}

	let {
		flowData,
		expandedRows,
		toggleExpanded,
		updateSelectedIteration,
		workspaceId,
		render,
		level = 0,
		flowId = 'root'
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

{#if render}
	<ul class="w-full font-mono text-xs bg-surface-secondary list-none">
		<!-- Start of flow -->
		<li class="border-b border-gray-200 dark:border-gray-700 flex">
			<div class="py-2 leading-tight align-top">
				<button
					class="w-4 flex items-center justify-center text-xs text-tertiary hover:text-primary transition-colors"
					onclick={() => toggleExpanded(`flow-start-${flowId}`)}
				>
					{#if expandedRows.has(`flow-start-${flowId}`)}
						<ChevronDown size={8} />
					{:else}
						<ChevronRight size={8} />
					{/if}
				</button>
			</div>
			<div class="w-full leading-tight">
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div
					class="py-1 flex items-center justify-between cursor-pointer"
					onclick={() => toggleExpanded(`flow-start-${flowId}`)}
				>
					<span class="text-xs font-mono">Running <b>flow</b></span>
					<a
						href={getJobLink(flowData.jobId)}
						class="text-xs text-primary hover:underline font-mono"
						target="_blank"
						rel="noopener noreferrer"
						onclick={(e) => e.stopPropagation()}
					>
						{truncateRev(flowData.jobId, 10)}
					</a>
				</div>

				{#if expandedRows.has(`flow-start-${flowId}`)}
					<div class="mt-1 pl-4 transition-all duration-200 ease-in-out">
						{#if flowData.inputs && Object.keys(flowData.inputs).length > 0}
							<div class="mb-2">
								<h4 class="text-xs font-mono font-medium mb-1">Input:</h4>
								<ObjectViewer json={flowData.inputs} pureViewer={true} />
							</div>
						{/if}
					</div>
				{/if}
			</div>
		</li>

		{#each logEntries as entry (entry.id)}
			<li class="border-b border-gray-200 dark:border-gray-700 flex">
				<div class="py-2 leading-tight align-top">
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
				</div>
				<div class="w-full leading-tight">
					<!-- svelte-ignore a11y_click_events_have_key_events -->
					<!-- svelte-ignore a11y_no_static_element_interactions -->
					<div
						class="py-1 flex items-center justify-between cursor-pointer"
						onclick={() => toggleExpanded(entry.id)}
					>
						<div class="flex items-center gap-2 grow min-w-0">
							{#if entry.type === 'start'}
								<div class="flex items-center gap-2">
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
											{:else if entry.stepType === 'flow'}
												flow
											{:else}
												step
											{/if}
											{entry.stepId}
										</b>
										{#if entry.summary}
											: {entry.summary}
										{/if}
									</span>
									{#if entry.stepData.subflows && (entry.stepType === 'forloopflow' || entry.stepType === 'whileloopflow')}
										<span
											class="text-xs font-mono font-medium inline-flex items-center gap-1 grow min-w-0"
										>
											iteration
											<select
												value={getSelectedIteration(entry.stepId)}
												onchange={(e) => {
													const target = e.target as HTMLSelectElement
													handleIterationChange(entry.stepId, Number(target.value))
												}}
												onclick={(e) => e.stopPropagation()}
												class="inline-block !-my-2 !w-12 !p-0.5 !text-xs bg-surface-secondary font-mono"
											>
												{#each entry.stepData.subflows as _, index}
													<option value={index}>
														{index + 1}
													</option>
												{/each}
											</select>
											{`/${entry.stepData.subflows.length}`}
										</span>
									{/if}
								</div>
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

						{#if entry.stepType !== 'branchall' && entry.stepType !== 'branchone' && entry.stepType !== 'forloopflow' && entry.stepType !== 'whileloopflow'}
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
						<div class="my-1 transition-all duration-200 ease-in-out">
							{#if entry.type === 'start'}
								{#if entry.stepData.subflows && entry.stepData.subflows.length > 0}
									{#if entry.stepType === 'forloopflow' || entry.stepType === 'whileloopflow'}
										{#if entry.stepData.subflows[getSelectedIteration(entry.stepId)]}
											<div class="border-l">
												<FlowLogViewer
													flowData={entry.stepData.subflows[getSelectedIteration(entry.stepId)]}
													{expandedRows}
													{toggleExpanded}
													{updateSelectedIteration}
													{workspaceId}
													{render}
													level={level + 1}
													flowId={entry.stepId}
												/>
											</div>
										{/if}
									{:else if entry.stepType === 'branchall' || entry.stepType === 'branchone'}
										<!-- This is a branch, show all subflows -->
										<div class="mb-2">
											{#each entry.stepData.subflows as subflow}
												<div class="mb-2 border-l">
													<FlowLogViewer
														flowData={subflow}
														{expandedRows}
														{toggleExpanded}
														{updateSelectedIteration}
														{workspaceId}
														{render}
														level={level + 1}
														flowId={subflow.jobId}
													/>
												</div>
											{/each}
										</div>
									{:else if entry.stepType === 'flow'}
										<div class="mb-2">
											<h4 class="text-xs font-mono font-medium mb-1">Input:</h4>
											<ObjectViewer json={entry.args} pureViewer={true} />
										</div>
										<div class="mb-2">
											<FlowLogViewer
												flowData={entry.stepData.subflows[0]}
												{expandedRows}
												{toggleExpanded}
												{updateSelectedIteration}
												{workspaceId}
												{render}
												level={level + 1}
												flowId={entry.stepId}
											/>
										</div>
									{/if}
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
								<div>
									<h4 class="text-xs font-mono font-medium mb-1">Result:</h4>
									{#if entry.result !== undefined}
										<ObjectViewer json={entry.result} pureViewer={true} />
									{:else}
										<!-- Todo: display in progress or waiting -->
										<div class="text-xs text-tertiary font-mono"> Null </div>
									{/if}
								</div>
							{/if}
						</div>
					{/if}
				</div>
			</li>
		{/each}

		<!-- End of flow -->
		<li class="border-gray-200 dark:border-gray-700 flex">
			<div class="py-2 leading-tight align-top">
				<button
					class="w-4 flex items-center justify-center text-xs text-tertiary hover:text-primary transition-colors"
					onclick={() => toggleExpanded(`flow-end-${flowId}`)}
				>
					{#if expandedRows.has(`flow-end-${flowId}`)}
						<ChevronDown size={8} />
					{:else}
						<ChevronRight size={8} />
					{/if}
				</button>
			</div>
			<div class="w-full leading-tight">
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div
					class="py-1 flex items-center justify-between cursor-pointer"
					onclick={() => toggleExpanded(`flow-end-${flowId}`)}
				>
					<span class="text-xs font-mono">
						Flow
						{#if flowData.status === 'success'}
							<span class="text-green-600">executed with success</span>
						{:else if flowData.status === 'failure'}
							<span class="text-red-600">failed</span>
						{:else if flowData.status === 'in_progress'}
							<span class="text-blue-600">in progress</span>
						{:else}
							<span class="text-gray-600">waiting</span>
						{/if}
					</span>
				</div>

				{#if expandedRows.has(`flow-end-${flowId}`)}
					<div class="mt-1 pl-4 transition-all duration-200 ease-in-out">
						{#if flowData.result !== undefined && (flowData.status === 'success' || flowData.status === 'failure')}
							<div class="mb-2">
								<h4 class="text-xs font-mono font-medium mb-1">Result:</h4>
								<ObjectViewer json={flowData.result} pureViewer={true} />
							</div>
						{/if}
					</div>
				{/if}
			</div>
		</li>
	</ul>
{/if}

<style>
	.transition-all {
		transition: all 0.2s ease-in-out;
	}
</style>
