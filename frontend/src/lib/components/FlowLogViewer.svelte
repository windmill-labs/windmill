<script lang="ts">
	import {
		ChevronDown,
		ChevronRight,
		GitBranch,
		Repeat,
		Code,
		ArrowDownToLine,
		ArrowDownFromLine,
		FoldVertical,
		UnfoldVertical
	} from 'lucide-svelte'
	import { base } from '$lib/base'
	import { workspaceStore } from '$lib/stores'
	import { truncateRev } from '$lib/utils'
	import ObjectViewer from './propertyPicker/ObjectViewer.svelte'
	import LogViewer from './LogViewer.svelte'
	import FlowLogViewer from './FlowLogViewer.svelte'
	import type { FlowModuleValue, FlowStatusModule, Job } from '$lib/gen'
	import { twMerge } from 'tailwind-merge'
	import FlowJobsMenu from './flows/map/FlowJobsMenu.svelte'
	import BarsStaggered from './icons/BarsStaggered.svelte'
	import type { GraphModuleState } from './graph/model'
	import type { Writable } from 'svelte/store'
	import type { FlowLogEntry } from './FlowLogUtils'

	type RootJobData = Partial<Job>

	interface Props {
		logEntries: FlowLogEntry[]
		localModuleStates: Writable<Record<string, GraphModuleState>>
		rootJob: RootJobData
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
		flowSummary?: string
	}

	let {
		logEntries,
		localModuleStates,
		rootJob,
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
		getSelectedIteration,
		flowSummary
	}: Props = $props()

	function getJobLink(jobId: string | undefined): string {
		if (!jobId) return ''
		return `${base}/run/${jobId}?workspace=${workspaceId ?? $workspaceStore}`
	}

	function getStatusColor(status: FlowStatusModule['type'] | undefined): string {
		const statusColors = {
			Success: 'text-green-500',
			Failure: 'text-red-500',
			InProgress: 'text-yellow-500',
			WaitingForPriorSteps: 'text-gray-400',
			WaitingForEvents: 'text-purple-400',
			WaitingForExecutor: 'text-gray-400'
		}
		return status ? statusColors[status] : 'text-gray-400'
	}

	function getFlowStatus(job: RootJobData): FlowStatusModule['type'] | undefined {
		if (job.type === 'CompletedJob') {
			return job.success ? 'Success' : 'Failure'
		} else if (job.type === 'QueuedJob') {
			return 'InProgress'
		} else {
			return undefined
		}
	}

	function getStepProgress(job: RootJobData, totalSteps: number): string {
		if (totalSteps === 0) return ''

		// If flow is completed, show total steps
		if (job.type === 'CompletedJob') {
			return ` (${totalSteps} step${totalSteps === 1 ? '' : 's'})`
		}

		// If flow is running, use flow_status.step if available (like JobStatus.svelte)
		if (job.type === 'QueuedJob') {
			if (job.flow_status?.step !== undefined) {
				const currentStep = (job.flow_status.step ?? 0) + 1
				return ` (step ${currentStep} of ${totalSteps})`
			}

			return ''
		}

		return ''
	}

	function isExpanded(id: string, isRunning: boolean = false): boolean {
		// If explicitly set in expandedRows, use that value
		// Otherwise, fall back to allExpanded
		return expandedRows[id] ?? (allExpanded || isRunning)
	}

	function hasEmptySubflow(stepId: string, stepType: FlowModuleValue['type'] | undefined): boolean {
		const state = $localModuleStates[stepId]

		if (!state || !stepType) return false
		return (
			['forloopflow', 'whileloopflow'].includes(stepType) &&
			(!state.flow_jobs || state.flow_jobs.length === 0)
		)
	}

	// Find all parents of error steps
	function findParentsOfErrors(entries: FlowLogEntry[]): Set<string> {
		const parentsWithErrors = new Set<string>()

		function traverseEntries(entryList: FlowLogEntry[], parentId?: string) {
			let hasChildError = false

			for (const entry of entryList) {
				let currentEntryHasError = false

				// Check if this entry has subflows with errors
				if (entry.subflows && entry.subflows.length > 0) {
					for (const subflow of entry.subflows) {
						const subflowHasError = traverseEntries(subflow, entry.stepId)
						if (subflowHasError) {
							currentEntryHasError = true
							parentsWithErrors.add(entry.stepId)
						}
					}
				}

				// Check if this entry itself has an error (but don't flag it - only its parents)
				const stepStatus = $localModuleStates[entry.stepId]?.type
				if (stepStatus === 'Failure') {
					currentEntryHasError = true
					// Don't add the entry itself to parentsWithErrors
				}

				// If this entry has an error, mark its parent
				if (currentEntryHasError && parentId) {
					parentsWithErrors.add(parentId)
					hasChildError = true
				}
			}

			return hasChildError
		}

		traverseEntries(entries, flowId)
		return parentsWithErrors
	}

	// Get flow info for display
	const flowInfo = $derived.by(() => {
		const parentsWithErrors = findParentsOfErrors(logEntries)
		return {
			jobId: rootJob.id,
			inputs: rootJob.args || {},
			result: rootJob.type === 'CompletedJob' ? rootJob.result : undefined,
			logs: rootJob.logs || '',
			status: rootJob.type,
			label: flowSummary,
			hasErrors: parentsWithErrors.has(flowId),
			parentsWithErrors
		}
	})
</script>

{#if render}
	{#if level === 0 && toggleExpandAll}
		<div class="flex justify-end gap-4 items-center p-2 bg-surface-secondary border-b">
			<div class="flex items-center gap-2 whitespace-nowrap">
				<label
					for="showResultsInputs"
					class="text-xs text-tertiary hover:text-primary transition-colors"
					>Show inputs/results</label
				>
				<div class="flex-shrink-0">
					<input
						type="checkbox"
						name="showResultsInputs"
						id="showResultsInputs"
						bind:checked={showResultsInputs}
						class="w-3 h-4 accent-primary -my-1"
					/>
				</div>
			</div>
			<button
				onclick={toggleExpandAll}
				class="text-xs text-tertiary hover:text-primary transition-colors flex items-center gap-2 min-w-24 justify-end"
			>
				{allExpanded ? 'Collapse All' : 'Expand All'}
				{#if allExpanded}
					<FoldVertical size={16} />
				{:else}
					<UnfoldVertical size={16} />
				{/if}
			</button>
		</div>
	{/if}
	<ul class="w-full font-mono text-xs bg-surface-secondary list-none">
		<!-- Flow entry -->
		<li class="border-b flex flex-row">
			<div class="py-2 leading-tight align-top">
				{#if level > 0}
					<button
						class="w-4 flex items-center justify-center text-xs text-tertiary hover:text-primary transition-colors"
						onclick={() => toggleExpanded(`flow-${flowId}`)}
					>
						{#if isExpanded(`flow-${flowId}`, rootJob.type === 'QueuedJob')}
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
			<div class="grow min-w-0 leading-tigh">
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div
					class={twMerge(
						'py-1 flex items-center justify-between pr-2',
						level > 0 ? 'cursor-pointer' : '',
						rootJob.type === undefined ? 'opacity-50' : ''
					)}
					onclick={level > 0 ? () => toggleExpanded(`flow-${flowId}`) : undefined}
				>
					<div class="flex items-center gap-2 grow min-w-0">
						<!-- Flow icon -->
						{@render flowIcon(getFlowStatus(rootJob), flowInfo.hasErrors)}

						<div class="flex items-center gap-2">
							<span class="text-xs font-mono">
								{flowId === 'root' ? 'Flow' : 'Subflow'}
								{#if flowInfo.label}
									: {flowInfo.label}
								{/if}
								<span class="text-tertiary">{getStepProgress(rootJob, logEntries.length)}</span>
							</span>
						</div>
					</div>

					{#if flowInfo.jobId}
						<a
							href={getJobLink(flowInfo.jobId)}
							class="text-xs text-primary hover:underline font-mono"
							target="_blank"
							rel="noopener noreferrer"
							onclick={(e) => e.stopPropagation()}
						>
							{truncateRev(flowInfo.jobId, 6)}
						</a>
					{/if}
				</div>

				{#if level === 0 || isExpanded(`flow-${flowId}`, rootJob.type === 'QueuedJob')}
					<div class="mb-2 transition-all duration-200 ease-in-out w-full">
						<!-- Flow logs -->
						{#if flowInfo.logs}
							<LogViewer
								content={flowInfo.logs}
								jobId={flowInfo.jobId}
								isLoading={false}
								small={true}
								download={false}
								noAutoScroll={true}
								tag={undefined}
								noPadding
								wrapperClass="w-full mb-2 pr-2"
							/>
						{/if}

						<!-- Flow steps - nested as children -->
						<ul class="w-full font-mono text-xs bg-surface-secondary list-none border-l">
							<!-- Flow inputs as first row entry -->
							{#if showResultsInputs && flowInfo.inputs && Object.keys(flowInfo.inputs).length > 0}
								<li class="border-b flex flex-row w-full">
									<div class="py-2 leading-tight align-top">
										<button
											class="w-4 flex items-center justify-center text-xs text-tertiary hover:text-primary transition-colors"
											onclick={() => toggleExpanded(`flow-${flowId}-input`)}
										>
											{#if isExpanded(`flow-${flowId}-input`)}
												<ChevronDown size={8} />
											{:else}
												<ChevronRight size={8} />
											{/if}
										</button>
									</div>

									<div class="grow min-w-0 leading-tight">
										<!-- svelte-ignore a11y_no_static_element_interactions -->
										<!-- svelte-ignore a11y_click_events_have_key_events -->
										<div
											class="py-1 flex items-center justify-between pr-2 cursor-pointer"
											onclick={() => toggleExpanded(`flow-${flowId}-input`)}
										>
											<div class="flex items-center gap-2 grow min-w-0">
												<ArrowDownToLine size={10} />
												<span class="text-xs font-mono">Inputs</span>
											</div>
										</div>

										{#if isExpanded(`flow-${flowId}-input`)}
											<div class="my-1 transition-all duration-200 ease-in-out">
												<div class="pl-4">
													<ObjectViewer json={flowInfo.inputs} pureViewer={true} />
												</div>
											</div>
										{/if}
									</div>
								</li>
							{/if}

							{#if logEntries.length > 0}
								{#each logEntries as entry (entry.id)}
									{@const isLeafStep =
										entry.stepType !== 'branchall' &&
										entry.stepType !== 'branchone' &&
										entry.stepType !== 'forloopflow' &&
										entry.stepType !== 'whileloopflow'}
									{@const status = $localModuleStates[entry.stepId]?.type}
									{@const isRunning = status === 'InProgress' || status === 'WaitingForExecutor'}
									{@const hasEmptySubflowValue = hasEmptySubflow(entry.stepId, entry.stepType)}
									{@const isCollapsible = !hasEmptySubflowValue}
									<li class="border border-b flex flex-row border-orange-500 w-full">
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
										<div class="w-full leading-tight grow min-w-0">
											<!-- svelte-ignore a11y_click_events_have_key_events -->
											<!-- svelte-ignore a11y_no_static_element_interactions -->
											<div
												class={twMerge(
													'py-1 flex items-center justify-between pr-2',
													isCollapsible ? 'cursor-pointer' : '',
													status === 'WaitingForPriorSteps' ||
														status === 'WaitingForEvents' ||
														status === 'WaitingForExecutor' ||
														status === undefined
														? 'opacity-50'
														: ''
												)}
												onclick={isCollapsible ? () => toggleExpanded(entry.id) : undefined}
											>
												<div class="flex items-center gap-2 grow min-w-0">
													<!-- Step icon -->
													{@render stepIcon(
														entry.stepType,
														status as FlowStatusModule['type'],
														flowInfo.parentsWithErrors.has(entry.stepId)
													)}

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
																Subflow
															{:else}
																Step
															{/if}
															{#if entry.summary}
																: {entry.summary}
															{/if}
															{#if hasEmptySubflowValue}
																<span class="text-tertiary">
																	{#if entry.stepType === 'forloopflow' || entry.stepType === 'whileloopflow'}
																		(empty loop)
																	{:else if entry.stepType === 'branchall' || entry.stepType === 'branchone'}
																		(no branch)
																	{/if}
																</span>
															{/if}
														</span>
														{#if !hasEmptySubflowValue && $localModuleStates[entry.stepId]?.flow_jobs && (entry.stepType === 'forloopflow' || entry.stepType === 'whileloopflow')}
															<span
																class="text-xs font-mono font-medium inline-flex items-center grow min-w-0 -my-2"
															>
																<span onclick={(e) => e.stopPropagation()}>
																	<FlowJobsMenu
																		moduleId={entry.stepId}
																		id={entry.stepId}
																		{onSelectedIteration}
																		flowJobsSuccess={$localModuleStates[entry.stepId]
																			?.flow_jobs_success}
																		flowJobs={$localModuleStates[entry.stepId]?.flow_jobs}
																		selected={$localModuleStates[entry.stepId]
																			?.selectedForloopIndex ?? 0}
																		selectedManually={$localModuleStates[entry.stepId]
																			?.selectedForLoopSetManually ?? false}
																		showIcon={false}
																	/>
																</span>
																{#if entry.stepType === 'forloopflow'}
																	{`/${$localModuleStates[entry.stepId]?.iteration_total ?? 0}`}
																{/if}
															</span>
														{/if}
													</div>
												</div>

												{#if isLeafStep}
													{@const jobId = $localModuleStates[entry.stepId]?.job_id}
													<a
														href={getJobLink(jobId ?? '')}
														class="text-xs text-primary hover:underline font-mono"
														target="_blank"
														rel="noopener noreferrer"
													>
														{truncateRev(jobId ?? '', 6)}
													</a>
												{/if}
											</div>

											{#if isCollapsible && isExpanded(entry.id, isRunning)}
												{@const args = $localModuleStates[entry.stepId]?.args}
												{@const logs = $localModuleStates[entry.stepId]?.logs}
												{@const result = $localModuleStates[entry.stepId]?.result}
												{@const jobId = $localModuleStates[entry.stepId]?.job_id}
												<div class="my-1 transition-all duration-200 ease-in-out">
													<!-- Show child steps if they exist -->
													{#if entry.subflows && entry.subflows.length > 0}
														{#each entry.subflows as subflow, index}
															{@const subflowLabel = entry.subflowsSummary?.[index]}
															{@const subflowJob = {
																id: jobId,
																type:
																	$localModuleStates[entry.stepId]?.type === 'Failure' ||
																	$localModuleStates[entry.stepId]?.type === 'Success'
																		? 'CompletedJob'
																		: ('QueuedJob' as Job['type']),
																logs,
																result,
																args,
																success: $localModuleStates[entry.stepId]?.type === 'Success'
															}}
															<div class="border-l mb-2">
																<!-- Recursively render child steps using FlowLogViewer -->
																<FlowLogViewer
																	logEntries={subflow}
																	{localModuleStates}
																	rootJob={subflowJob}
																	{expandedRows}
																	{allExpanded}
																	{showResultsInputs}
																	{toggleExpanded}
																	toggleExpandAll={undefined}
																	{workspaceId}
																	{render}
																	level={level + 1}
																	flowId={`${entry.stepId}-subflow-${index}`}
																	flowSummary={subflowLabel}
																	{onSelectedIteration}
																	{getSelectedIteration}
																/>
															</div>
														{/each}
														<!-- Show input arguments -->
													{:else}
														{#if showResultsInputs && isLeafStep && args && Object.keys(args).length > 0}
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
																		<ObjectViewer json={args} pureViewer={true} />
																	</div>
																{/if}
															</div>
														{/if}

														<!-- Show logs if they exist -->
														{#if logs}
															<LogViewer
																content={logs}
																jobId={jobId ?? ''}
																isLoading={false}
																small={true}
																download={false}
																noAutoScroll={true}
																tag={undefined}
																noPadding
																wrapperClass="w-full mb-2 pr-2"
															/>
														{:else if jobId && !entry.subflows?.[0]?.length}
															<div class="mb-2">
																<div class="text-xs text-tertiary font-mono">
																	No logs available
																</div>
															</div>
														{/if}

														<!-- Show result if completed -->

														{#if showResultsInputs && isLeafStep && result !== undefined && (status === 'Success' || status === 'Failure')}
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
																		<ObjectViewer json={result} pureViewer={true} />
																	</div>
																{/if}
															</div>
														{/if}
													{/if}
												</div>
											{/if}
										</div>
									</li>
								{/each}
							{/if}

							<!-- Flow result as last row entry -->
							{#if showResultsInputs && flowInfo.result !== undefined && rootJob.type === 'CompletedJob'}
								<li class="border-b border-gray-200 dark:border-gray-700 flex">
									<div class="py-2 leading-tight align-top">
										<button
											class="w-4 flex items-center justify-center text-xs text-tertiary hover:text-primary transition-colors"
											onclick={() => toggleExpanded(`flow-${flowId}-result`)}
										>
											{#if isExpanded(`flow-${flowId}-result`)}
												<ChevronDown size={8} />
											{:else}
												<ChevronRight size={8} />
											{/if}
										</button>
									</div>
									<div class="w-full leading-tight">
										<!-- svelte-ignore a11y_no_static_element_interactions -->
										<!-- svelte-ignore a11y_click_events_have_key_events -->
										<div
											class="py-1 flex items-center justify-between pr-2 cursor-pointer"
											onclick={() => toggleExpanded(`flow-${flowId}-result`)}
										>
											<div class="flex items-center gap-2 grow min-w-0">
												<ArrowDownFromLine size={10} />
												<span class="text-xs font-mono">Results</span>
											</div>
										</div>

										{#if isExpanded(`flow-${flowId}-result`)}
											<div class="my-1 transition-all duration-200 ease-in-out">
												<div class="pl-4">
													<ObjectViewer json={flowInfo.result} pureViewer={true} />
												</div>
											</div>
										{/if}
									</div>
								</li>
							{/if}
						</ul>
					</div>
				{/if}
			</div>
		</li>
	</ul>
{/if}

{#snippet flowIcon(status: FlowStatusModule['type'] | undefined, hasErrors: boolean)}
	{@const colorClass = getStatusColor(status)}
	<div class="relative flex items-center">
		<BarsStaggered
			size={10}
			class={twMerge(colorClass, status === 'InProgress' ? 'animate-pulse' : '', 'flex-shrink-0')}
		/>
		{#if hasErrors && status !== 'Failure'}
			<span
				class="text-red-500 -ml-0.5 -mr-1.5"
				title="A subflow or a step has failed but failure was skipped">!</span
			>
		{/if}
	</div>
{/snippet}

{#snippet stepIcon(
	stepType: string | undefined,
	status: FlowStatusModule['type'] | undefined,
	hasErrors: boolean
)}
	{@const colorClass = getStatusColor(status)}
	{@const animationClass = status === 'InProgress' ? 'animate-pulse' : ''}
	{@const classes = `${colorClass} ${animationClass} flex-shrink-0`}
	<div class="relative flex items-center">
		{#if stepType === 'flow'}
			<BarsStaggered size={10} class={classes} />
		{:else if stepType === 'forloopflow' || stepType === 'whileloopflow'}
			<Repeat size={10} class={classes} />
		{:else if stepType === 'branchall' || stepType === 'branchone'}
			<GitBranch size={10} class={classes} />
		{:else}
			<Code strokeWidth={2.5} size={10} class={classes} />
		{/if}
		{#if hasErrors && status !== 'Failure'}
			<span class="text-red-500 -ml-0.5 -mr-1.5" title="A subflow or a step has failed">!</span>
		{/if}
	</div>
{/snippet}

<style>
	.transition-all {
		transition: all 0.2s ease-in-out;
	}
</style>
