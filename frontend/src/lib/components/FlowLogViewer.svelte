<script lang="ts">
	import { ChevronDown, ChevronRight } from 'lucide-svelte'
	import { base } from '$lib/base'
	import { workspaceStore } from '$lib/stores'
	import { truncateRev } from '$lib/utils'
	import ObjectViewer from './propertyPicker/ObjectViewer.svelte'
	import LogViewer from './LogViewer.svelte'
	import FlowLogViewer from './FlowLogViewer.svelte'
	import type { FlowData, StepData } from './FlowLogUtils'
	import type { FlowStatusModule } from '$lib/gen'

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

	function getStatusDot(status: FlowStatusModule['type'] | undefined) {
		const statusClasses = {
			Success: 'bg-green-500',
			Failure: 'bg-red-500',
			InProgress: 'bg-yellow-500 animate-pulse',
			WaitingForPriorSteps: 'bg-gray-400',
			WaitingForEvents: 'bg-purple-400',
			WaitingForExecutor: 'bg-gray-400'
		}
		return status ? statusClasses[status] : 'bg-gray-400'
	}

	// Create log entries for display - one entry per step
	interface LogEntry {
		id: string
		stepId: string
		stepNumber: number
		jobId: string
		status: FlowStatusModule['type']
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
			// Create single entry per step
			entries.push({
				id: step.stepId,
				stepId: step.stepId,
				stepNumber: step.stepNumber,
				jobId: step.jobId || '',
				status: step.status,
				stepData: step,
				args: step.inputs,
				result: step.result,
				logs: step.logs || '',
				summary: step.summary,
				stepType: step.type
			})
		})

		return entries
	}

	let logEntries = $derived(createLogEntries(flowData.steps))

	function getFlowStatus(flowData: FlowData): FlowStatusModule['type'] | undefined {
		if (flowData.status === 'CompletedJob') {
			return flowData.success ? 'Success' : 'Failure'
		} else if (flowData.status === 'QueuedJob') {
			return 'InProgress'
		} else {
			return undefined
		}
	}
</script>

{#if render}
	<ul class="w-full font-mono text-xs bg-surface-secondary list-none">
		<!-- Flow entry -->
		<li class="border-b border-gray-200 dark:border-gray-700 flex">
			<div class="py-2 leading-tight align-top">
				<button
					class="w-4 flex items-center justify-center text-xs text-tertiary hover:text-primary transition-colors"
					onclick={() => toggleExpanded(`flow-${flowId}`)}
				>
					{#if expandedRows.has(`flow-${flowId}`)}
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
					class="py-1 flex items-center justify-between cursor-pointer {flowData.status ===
					'QueuedJob'
						? 'opacity-50'
						: ''}"
					onclick={() => toggleExpanded(`flow-${flowId}`)}
				>
					<div class="flex items-center gap-2 grow min-w-0">
						<!-- Status dot -->
						<div class="w-1.5 h-1.5 rounded-full {getStatusDot(getFlowStatus(flowData))}"></div>

						<div class="flex items-center gap-2">
							<span class="text-xs font-mono">
								<b>flow</b>
							</span>
						</div>
					</div>

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

				{#if expandedRows.has(`flow-${flowId}`)}
					<div class="my-1 transition-all duration-200 ease-in-out">
						<!-- Show flow input arguments -->
						{#if flowData.inputs && Object.keys(flowData.inputs).length > 0}
							<div class="mb-2">
								<!-- svelte-ignore a11y_click_events_have_key_events -->
								<!-- svelte-ignore a11y_no_static_element_interactions -->
								<div
									class="flex items-center gap-1 cursor-pointer hover:text-primary text-xs font-mono font-medium mb-1"
									onclick={() => toggleExpanded(`flow-${flowId}-input`)}
								>
									{#if expandedRows.has(`flow-${flowId}-input`)}
										<ChevronDown size={10} />
									{:else}
										<ChevronRight size={10} />
									{/if}
									Input:
								</div>
								{#if expandedRows.has(`flow-${flowId}-input`)}
									<div class="pl-4">
										<ObjectViewer json={flowData.inputs} pureViewer={true} />
									</div>
								{/if}
							</div>
						{/if}

						<!-- Flow steps will be rendered here by the main loop -->
						{#if flowData.logs}
							<div class="mb-2">
								<LogViewer
									content={flowData.logs}
									jobId={flowData.jobId}
									isLoading={false}
									small={true}
									download={false}
									noAutoScroll={true}
									tag={undefined}
									noPadding
								/>
							</div>
						{/if}

						<!-- Show flow result if completed -->
						{#if flowData.result !== undefined && flowData.status === 'CompletedJob'}
							<div class="mb-2 mt-2">
								<!-- svelte-ignore a11y_click_events_have_key_events -->
								<!-- svelte-ignore a11y_no_static_element_interactions -->
								<div
									class="flex items-center gap-1 cursor-pointer hover:text-primary text-xs font-mono font-medium mb-1"
									onclick={() => toggleExpanded(`flow-${flowId}-result`)}
								>
									{#if expandedRows.has(`flow-${flowId}-result`)}
										<ChevronDown size={10} />
									{:else}
										<ChevronRight size={10} />
									{/if}
									Result:
								</div>
								{#if expandedRows.has(`flow-${flowId}-result`)}
									<div class="pl-4">
										<ObjectViewer json={flowData.result} pureViewer={true} />
									</div>
								{/if}
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
						class="py-1 flex items-center justify-between cursor-pointer {entry.status ===
							'WaitingForPriorSteps' ||
						entry.status === 'WaitingForEvents' ||
						entry.status === 'WaitingForExecutor'
							? 'opacity-50'
							: ''}"
						onclick={() => toggleExpanded(entry.id)}
					>
						<div class="flex items-center gap-2 grow min-w-0">
							<!-- Status dot -->
							<div class="w-1.5 h-1.5 rounded-full {getStatusDot(entry.status)} flex-shrink-0"
							></div>

							<div class="flex items-center gap-2">
								<span class="text-xs font-mono">
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
							<!-- Show input arguments -->
							<!-- Todo: fetch inputs for iterator, branch conditions, etc. -->
							{#if entry.args && Object.keys(entry.args).length > 0}
								<div class="mb-2">
									<!-- svelte-ignore a11y_click_events_have_key_events -->
									<!-- svelte-ignore a11y_no_static_element_interactions -->
									<div
										class="flex items-center gap-1 cursor-pointer hover:text-primary text-xs font-mono font-medium mb-1"
										onclick={() => toggleExpanded(`${entry.id}-input`)}
									>
										{#if expandedRows.has(`${entry.id}-input`)}
											<ChevronDown size={10} />
										{:else}
											<ChevronRight size={10} />
										{/if}
										Input:
									</div>
									{#if expandedRows.has(`${entry.id}-input`)}
										<div class="pl-4">
											<ObjectViewer json={entry.args} pureViewer={true} />
										</div>
									{/if}
								</div>
							{/if}

							<!-- Show subflows for complex step types -->
							{#if entry.stepData.subflows && entry.stepData.subflows.length > 0}
								{#if entry.stepType === 'forloopflow' || entry.stepType === 'whileloopflow'}
									{#if entry.stepData.subflows[getSelectedIteration(entry.stepId)]}
										<div class="border-l mb-2">
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
							{:else if entry.logs}
								<div class="mb-2">
									<LogViewer
										content={entry.logs}
										jobId={entry.jobId}
										isLoading={false}
										small={true}
										download={false}
										noAutoScroll={true}
										tag={undefined}
										noPadding
									/>
								</div>
							{:else if entry.jobId && !entry.stepData.subflows}
								<div class="mb-2">
									<div class="text-xs text-tertiary font-mono"> No logs available </div>
								</div>
							{/if}

							<!-- Show result if completed -->
							<!-- Todo: show result for subflows -->
							{#if entry.result !== undefined && (entry.status === 'Success' || entry.status === 'Failure')}
								<div class="mb-2 mt-2">
									<!-- svelte-ignore a11y_click_events_have_key_events -->
									<!-- svelte-ignore a11y_no_static_element_interactions -->
									<div
										class="flex items-center gap-1 cursor-pointer hover:text-primary text-xs font-mono font-medium mb-1"
										onclick={() => toggleExpanded(`${entry.id}-result`)}
									>
										{#if expandedRows.has(`${entry.id}-result`)}
											<ChevronDown size={10} />
										{:else}
											<ChevronRight size={10} />
										{/if}
										Result:
									</div>
									{#if expandedRows.has(`${entry.id}-result`)}
										<div class="pl-4">
											<ObjectViewer json={entry.result} pureViewer={true} />
										</div>
									{/if}
								</div>
							{/if}
						</div>
					{/if}
				</div>
			</li>
		{/each}
	</ul>
{/if}

<style>
	.transition-all {
		transition: all 0.2s ease-in-out;
	}
</style>
