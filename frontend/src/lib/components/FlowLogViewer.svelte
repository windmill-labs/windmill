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
	import { twMerge } from 'tailwind-merge'

	interface Props {
		flowData: FlowData
		expandedRows: Record<string, boolean>
		allExpanded?: boolean
		showResultsInputs?: boolean
		toggleExpanded: (id: string) => void
		toggleExpandAll?: () => void
		workspaceId: string | undefined
		render: boolean
		level?: number
		flowId: string
		onSelectedIteration: (
			detail:
				| { id: string; index: number; manuallySet: true; moduleId: string }
				| { manuallySet: false; moduleId: string }
		) => Promise<void>
		getSelectedIteration: (stepId: string) => number
	}

	let {
		flowData,
		expandedRows,
		allExpanded,
		showResultsInputs,
		toggleExpanded,
		toggleExpandAll,
		workspaceId,
		render,
		level = 0,
		flowId = 'root',
		onSelectedIteration,
		getSelectedIteration
	}: Props = $props()

	function getJobLink(jobId: string): string {
		return `${base}/run/${jobId}?workspace=${workspaceId ?? $workspaceStore}`
	}

	async function handleIterationChange(stepId: string, newIteration: number, jobId: string) {
		await onSelectedIteration({
			id: jobId,
			index: newIteration,
			manuallySet: true,
			moduleId: stepId
		})
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

	function isExpanded(id: string, isRunning: boolean = false): boolean {
		// If explicitly set in expandedRows, use that value
		// Otherwise, fall back to allExpanded
		return expandedRows[id] ?? (allExpanded || isRunning)
	}
</script>

{#if render}
	{#if level === 0 && toggleExpandAll}
		<div class="flex justify-end gap-4 items-center p-2 bg-surface-secondary border-b">
			<div class="flex items-center gap-2 whitespace-nowrap">
				<label for="showResultsInputs" class="text-xs text-tertiary">Show inputs/results</label>
				<div class="flex-shrink-0">
					<input
						type="checkbox"
						name="showResultsInputs"
						id="showResultsInputs"
						bind:checked={showResultsInputs}
						class="w-3 h-3 accent-primary"
					/>
				</div>
			</div>
			<button
				onclick={toggleExpandAll}
				class="text-xs text-tertiary hover:text-primary transition-colors underline"
			>
				{allExpanded ? 'Collapse All' : 'Expand All'}
			</button>
		</div>
	{/if}
	<ul class="w-full font-mono text-xs bg-surface-secondary list-none">
		<!-- Flow entry -->
		<li class="border-b border-gray-200 dark:border-gray-700 flex">
			<div class="py-2 leading-tight align-top">
				{#if level > 0}
					<button
						class="w-4 flex items-center justify-center text-xs text-tertiary hover:text-primary transition-colors"
						onclick={() => toggleExpanded(`flow-${flowId}`)}
					>
						{#if isExpanded(`flow-${flowId}`, flowData.status === 'QueuedJob')}
							<ChevronDown size={8} />
						{:else}
							<ChevronRight size={8} />
						{/if}
					</button>
				{:else}
					<!-- Root flow - no collapse button, just spacing -->
					<div class="w-4"></div>
				{/if}
			</div>
			<div class="w-full leading-tight">
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div
					class="py-1 flex items-center justify-between {level > 0
						? 'cursor-pointer'
						: ''} {flowData.status === undefined ? 'opacity-50' : ''}"
					onclick={level > 0 ? () => toggleExpanded(`flow-${flowId}`) : undefined}
				>
					<div class="flex items-center gap-2 grow min-w-0">
						<!-- Status dot -->
						<div class="w-1.5 h-1.5 rounded-full {getStatusDot(getFlowStatus(flowData))}"></div>

						<div class="flex items-center gap-2">
							<span class="text-xs font-mono">
								<b>Flow</b>
								{#if flowData.label}
									: {flowData.label}
								{/if}
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

				{#if level === 0 || isExpanded(`flow-${flowId}`, flowData.status === 'QueuedJob')}
					<div class="mb-2 transition-all duration-200 ease-in-out">
						<!-- Show flow input arguments -->
						{#if showResultsInputs && flowData.inputs && Object.keys(flowData.inputs).length > 0}
							<div class="mb-2">
								<!-- svelte-ignore a11y_click_events_have_key_events -->
								<!-- svelte-ignore a11y_no_static_element_interactions -->
								<div
									class="flex items-center gap-1 cursor-pointer hover:text-primary text-xs font-mono font-medium mb-1"
									onclick={() => toggleExpanded(`flow-${flowId}-input`)}
								>
									{#if isExpanded(`flow-${flowId}-input`)}
										<ChevronDown size={8} />
									{:else}
										<ChevronRight size={8} />
									{/if}
									Inputs
								</div>
								{#if isExpanded(`flow-${flowId}-input`)}
									<div class="pl-4">
										<ObjectViewer json={flowData.inputs} pureViewer={true} />
									</div>
								{/if}
							</div>
						{/if}

						<!-- Flow logs -->
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

						<!-- Flow steps - nested as children -->
						{#if logEntries.length > 0}
							<ul
								class="w-full font-mono text-xs bg-surface-secondary list-none border-l border-gray-200 dark:border-gray-700"
							>
								{#each logEntries as entry (entry.id)}
									{@const isLeafStep =
										entry.stepType !== 'branchall' &&
										entry.stepType !== 'branchone' &&
										entry.stepType !== 'forloopflow' &&
										entry.stepType !== 'whileloopflow'}
									{@const isRunning =
										entry.status === 'InProgress' || entry.status === 'WaitingForExecutor'}
									{@const hasEmptySubflow = entry.stepData.emptySubflow === true}
									{@const isCollapsible = !hasEmptySubflow}
									<li class="border-b border-gray-200 dark:border-gray-700 flex">
										<div class="py-2 leading-tight align-top">
											{#if isCollapsible}
												<button
													class="w-4 flex items-center justify-center text-xs text-tertiary hover:text-primary transition-colors"
													onclick={() => toggleExpanded(entry.id)}
												>
													{#if isExpanded(entry.id, isRunning)}
														<ChevronDown size={8} />
													{:else}
														<ChevronRight size={8} />
													{/if}
												</button>
											{:else}
												<!-- Empty subflow - no collapse button, just spacing -->
												<div class="w-4"></div>
											{/if}
										</div>
										<div class="w-full leading-tight">
											<!-- svelte-ignore a11y_click_events_have_key_events -->
											<!-- svelte-ignore a11y_no_static_element_interactions -->
											<div
												class={twMerge(
													'py-1 flex items-center justify-between',
													isCollapsible ? 'cursor-pointer' : '',
													entry.status === 'WaitingForPriorSteps' ||
														entry.status === 'WaitingForEvents' ||
														entry.status === 'WaitingForExecutor' ||
														entry.status === undefined
														? 'opacity-50'
														: ''
												)}
												onclick={isCollapsible ? () => toggleExpanded(entry.id) : undefined}
											>
												<div class="flex items-center gap-2 grow min-w-0">
													<!-- Status dot -->
													<div
														class="w-1.5 h-1.5 rounded-full {getStatusDot(
															entry.status
														)} flex-shrink-0"
													></div>

													<div class="flex items-center gap-2">
														<span class="text-xs font-mono">
															<b>
																{entry.stepId}
															</b>
															{#if entry.stepType === 'forloopflow'}
																For loop
															{:else if entry.stepType === 'whileloopflow'}
																While loop
															{:else if entry.stepType === 'branchall'}
																Branch to all
															{:else if entry.stepType === 'branchone'}
																Branch to one
															{:else if entry.stepType === 'flow'}
																Flow
															{:else}
																Step
															{/if}
															{#if entry.summary}
																: {entry.summary}
															{/if}
															{#if hasEmptySubflow}
																<span class="text-tertiary">
																	{#if entry.stepType === 'forloopflow' || entry.stepType === 'whileloopflow'}
																		(empty loop)
																	{:else if entry.stepType === 'branchall' || entry.stepType === 'branchone'}
																		(no branch)
																	{/if}
																</span>
															{/if}
														</span>
														{#if !hasEmptySubflow && entry.stepData.subflows && (entry.stepType === 'forloopflow' || entry.stepType === 'whileloopflow')}
															<span
																class="text-xs font-mono font-medium inline-flex items-center gap-1 grow min-w-0"
															>
																iteration
																<select
																	value={getSelectedIteration(entry.stepId)}
																	onchange={(e) => {
																		const target = e.target as HTMLSelectElement
																		handleIterationChange(
																			entry.stepId,
																			Number(target.value),
																			entry.stepData.subflows?.[Number(target.value)]?.jobId ?? ''
																		)
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
																{`/${entry.stepData.iterationTotal}`}
															</span>
														{/if}
													</div>
												</div>

												{#if isLeafStep}
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

											{#if isCollapsible && isExpanded(entry.id, isRunning)}
												<div class="my-1 transition-all duration-200 ease-in-out">
													<!-- Show input arguments -->
													<!-- Todo: fetch inputs for iterator, branch conditions, etc. -->
													{#if showResultsInputs && isLeafStep && entry.args && Object.keys(entry.args).length > 0}
														<div class="mb-2">
															<!-- svelte-ignore a11y_click_events_have_key_events -->
															<!-- svelte-ignore a11y_no_static_element_interactions -->
															<div
																class="flex items-center gap-1 cursor-pointer hover:text-primary text-xs font-mono font-medium mb-1"
																onclick={() => toggleExpanded(`${entry.id}-input`)}
															>
																{#if isExpanded(`${entry.id}-input`)}
																	<ChevronDown size={8} />
																{:else}
																	<ChevronRight size={8} />
																{/if}
																Input
															</div>
															{#if isExpanded(`${entry.id}-input`)}
																<div class="pl-4">
																	<ObjectViewer json={entry.args} pureViewer={true} />
																</div>
															{/if}
														</div>
													{/if}

													<!-- Show subflows for complex step types -->
													{#if !hasEmptySubflow && entry.stepData.subflows && entry.stepData.subflows.length > 0}
														{#if entry.stepType === 'forloopflow' || entry.stepType === 'whileloopflow'}
															{#if entry.stepData.subflows[getSelectedIteration(entry.stepId)]}
																<div class="border-l mb-2">
																	<FlowLogViewer
																		flowData={entry.stepData.subflows[
																			getSelectedIteration(entry.stepId)
																		]}
																		{expandedRows}
																		{allExpanded}
																		{showResultsInputs}
																		{toggleExpanded}
																		toggleExpandAll={undefined}
																		{onSelectedIteration}
																		{getSelectedIteration}
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
																	<div class="mb-1 border-l">
																		<FlowLogViewer
																			flowData={subflow}
																			{expandedRows}
																			{allExpanded}
																			{showResultsInputs}
																			{toggleExpanded}
																			toggleExpandAll={undefined}
																			{onSelectedIteration}
																			{getSelectedIteration}
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
																	{allExpanded}
																	{showResultsInputs}
																	{toggleExpanded}
																	toggleExpandAll={undefined}
																	{onSelectedIteration}
																	{getSelectedIteration}
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
													{#if showResultsInputs && isLeafStep && entry.result !== undefined && (entry.status === 'Success' || entry.status === 'Failure')}
														<div class="mb-2 mt-2">
															<!-- svelte-ignore a11y_click_events_have_key_events -->
															<!-- svelte-ignore a11y_no_static_element_interactions -->
															<div
																class="flex items-center gap-1 cursor-pointer hover:text-primary text-xs font-mono font-medium mb-1"
																onclick={() => toggleExpanded(`${entry.id}-result`)}
															>
																{#if isExpanded(`${entry.id}-result`)}
																	<ChevronDown size={8} />
																{:else}
																	<ChevronRight size={8} />
																{/if}
																Result
															</div>
															{#if isExpanded(`${entry.id}-result`)}
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

						<!-- Show flow result if completed -->
						{#if showResultsInputs && flowData.result !== undefined && flowData.status === 'CompletedJob'}
							<div class="mb-2 mt-2">
								<!-- svelte-ignore a11y_click_events_have_key_events -->
								<!-- svelte-ignore a11y_no_static_element_interactions -->
								<div
									class="flex items-center gap-1 cursor-pointer hover:text-primary text-xs font-mono font-medium mb-1"
									onclick={() => toggleExpanded(`flow-${flowId}-result`)}
								>
									{#if isExpanded(`flow-${flowId}-result`)}
										<ChevronDown size={8} />
									{:else}
										<ChevronRight size={8} />
									{/if}
									Result
								</div>
								{#if isExpanded(`flow-${flowId}-result`)}
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
	</ul>
{/if}

<style>
	.transition-all {
		transition: all 0.2s ease-in-out;
	}
</style>
