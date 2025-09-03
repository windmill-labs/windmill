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
	import type { FlowModule, FlowModuleValue, FlowStatusModule, Job } from '$lib/gen'
	import { twMerge } from 'tailwind-merge'
	import FlowJobsMenu from './flows/map/FlowJobsMenu.svelte'
	import BarsStaggered from './icons/BarsStaggered.svelte'
	import type { GraphModuleState } from './graph/model'
	import type { NavigationChain } from '$lib/keyboardChain'
	import { updateLinks } from '$lib/keyboardChain'

	type RootJobData = Partial<Job>

	interface Props {
		modules: FlowModule[]
		localModuleStates: Record<string, GraphModuleState>
		rootJob: RootJobData
		flowStatus: FlowStatusModule['type'] | undefined
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
		mode?: 'flow' | 'aiagent'
		currentId?: string | null
		navigationChain?: NavigationChain
	}

	let {
		modules,
		localModuleStates,
		rootJob,
		flowStatus,
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
		flowSummary,
		mode = 'flow',
		currentId,
		navigationChain = $bindable()
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

		const stepWord = mode === 'aiagent' ? 'action' : 'step'

		// If flow is completed, show total steps
		if (job.type === 'CompletedJob') {
			return ` (${totalSteps} ${stepWord}${totalSteps === 1 ? '' : 's'})`
		}

		// If flow is running, use flow_status.step if available (like JobStatus.svelte)
		if (job.type === 'QueuedJob') {
			if (job.flow_status?.step !== undefined) {
				const currentStep = (job.flow_status.step ?? 0) + 1
				return ` (${stepWord} ${currentStep} of ${totalSteps})`
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
		const state = localModuleStates[stepId]

		if (!state || !stepType) return false
		return (
			['forloopflow', 'whileloopflow'].includes(stepType) &&
			(!state.flow_jobs || state.flow_jobs.length === 0)
		)
	}

	// Find all parents of error steps
	function findParentsOfErrors(modules: FlowModule[]): Set<string> {
		const parentsWithErrors = new Set<string>()

		function traverseModules(modules: FlowModule[], parentId?: string) {
			let hasChildError = false

			for (const module of modules) {
				let currentEntryHasError = false

				// Check if this entry has subflows with errors
				if (
					module.value.type === 'forloopflow' ||
					(module.value.type === 'whileloopflow' && module.value.modules.length > 0)
				) {
					const subflowHasError = traverseModules(module.value.modules, module.id)
					if (subflowHasError) {
						currentEntryHasError = true
						parentsWithErrors.add(module.id)
					}
				} else if (module.value.type === 'branchone' || module.value.type === 'branchall') {
					if (module.value.branches.length > 0) {
						for (const branch of module.value.branches) {
							const subflowHasError = traverseModules(branch.modules, module.id)
							if (subflowHasError) {
								currentEntryHasError = true
								parentsWithErrors.add(module.id)
							}
						}
					}
					if (module.value.type === 'branchone' && module.value.default.length > 0) {
						// Also check default branch
						const subflowHasError = traverseModules(module.value.default, module.id)
						if (subflowHasError) {
							currentEntryHasError = true
							parentsWithErrors.add(module.id)
						}
					}
				}

				// Check if this entry itself has an error (but don't flag it - only its parents)
				const stepStatus = localModuleStates[module.id]?.type
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

		traverseModules(modules, flowId)
		return parentsWithErrors
	}

	// Get flow info for display
	const flowInfo = $derived.by(() => {
		const parentsWithErrors = findParentsOfErrors(modules)
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

	let subloopNavigationChains = $state<Record<string, NavigationChain>>({})

	// Build navigation links directly (merged function)
	function buildNavigationLinks(): NavigationChain {
		const items: string[] = []

		// Flow header (always first)
		items.push(`flow-${flowId}`)

		if (level > 0 && !isExpanded(`flow-${flowId}`)) {
			return { [`flow-${flowId}`]: { upId: null, downId: null } }
		}

		// Flow input (if exists and shown)
		if (showResultsInputs && flowInfo.inputs && Object.keys(flowInfo.inputs).length > 0) {
			items.push(`flow-${flowId}-input`)
		}

		// Add modules in order
		modules.forEach((module) => {
			items.push(module.id)

			// If module is expanded, add its children
			if (isExpanded(module.id)) {
				// Check if it has subflows
				const subflows = getSubflows(module)
				if (subflows.length > 0) {
					// Add subflow IDs
					subflows.forEach((subflow) => {
						items.push(`flow-${subflow.flowId}`)
					})
				} else {
					// Add input if exists and shown (leaf nodes only)
					if (
						showResultsInputs &&
						localModuleStates[module.id]?.args &&
						Object.keys(localModuleStates[module.id].args).length > 0
					) {
						items.push(`${module.id}-input`)
					}

					// Add result if exists and shown (leaf nodes only)
					if (showResultsInputs && localModuleStates[module.id]?.result !== undefined) {
						items.push(`${module.id}-result`)
					}
				}
			}
		})

		// Flow result (if exists and shown)
		if (showResultsInputs && flowInfo.result !== undefined && rootJob.type === 'CompletedJob') {
			items.push(`flow-${flowId}-result`)
		}

		// Convert items to navigation links
		let links = items.reduce((acc, itemId, index) => {
			let upId: string | null = index > 0 ? items[index - 1] : null
			let downId: string | null = index < items.length - 1 ? items[index + 1] : null

			acc[itemId] = { upId, downId }
			return acc
		}, {} as NavigationChain)

		// Add subflow chains
		Object.entries(subloopNavigationChains).forEach(([_, chain]) => {
			links = updateLinks($state.snapshot(links), $state.snapshot(chain))
		})

		return links
	}

	const links = $derived.by(buildNavigationLinks)

	$effect(() => {
		navigationChain = links
	})

	// Helper to check if an item is currently focused
	function isCurrent(itemId: string): boolean {
		return currentId === itemId
	}

	// Scroll current item into view
	$effect(() => {
		if (currentId) {
			const element = document.querySelector(`[data-nav-id="${currentId}"]`)
			if (element) {
				element.scrollIntoView({ behavior: 'smooth', block: 'nearest' })
			}
		}
	})

	function hasSubflows(module: FlowModule) {
		return (
			module.value.type === 'forloopflow' ||
			module.value.type === 'whileloopflow' ||
			module.value.type === 'branchall' ||
			module.value.type === 'branchone'
		)
	}

	function getSubflows(module: FlowModule) {
		const subflows: Array<{ modules: FlowModule[]; label: string; flowId: string }> = []

		if (module.value.type === 'forloopflow' || module.value.type === 'whileloopflow') {
			subflows.push({
				modules: module.value.modules,
				label: module.summary || '',
				flowId: `${module.id}-subflow`
			})
		} else if (module.value.type === 'branchall' || module.value.type === 'branchone') {
			// Add all branches
			for (let i = 0; i < module.value.branches.length; i++) {
				const branch = module.value.branches[i]
				subflows.push({
					modules: branch.modules,
					label: branch.summary || `branch ${i + 1}`,
					flowId: `${module.id}-subflow-${i}`
				})
			}
			// Add default branch for branchone
			if (module.value.type === 'branchone') {
				subflows.push({
					modules: module.value.default,
					label: 'default',
					flowId: `${module.id}-subflow-default`
				})
			}
		}

		return subflows
	}
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
			{@render collapsibleButton(`flow-${flowId}`, level > 0, rootJob.type === 'QueuedJob')}

			<div class="grow min-w-0 leading-tight">
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div
					class={twMerge(
						'py-1 flex items-center justify-between pr-2',
						level > 0 ? 'cursor-pointer' : '',
						rootJob.type === undefined ? 'opacity-50' : '',
						isCurrent(`flow-${flowId}`) ? 'bg-surface-hover' : ''
					)}
					onclick={level > 0 ? () => toggleExpanded(`flow-${flowId}`) : undefined}
					data-nav-id={`flow-${flowId}`}
				>
					<div class="flex items-center gap-2 grow min-w-0">
						<!-- Flow icon -->
						{@render flowIcon(level == 0 ? getFlowStatus(rootJob) : flowStatus, flowInfo.hasErrors)}

						<div class="flex items-center gap-2">
							<span class="text-xs font-mono">
								{mode === 'aiagent' ? 'AI Agent' : level == 0 ? 'Flow' : 'Subflow'}
								{#if flowInfo.label}
									: {flowInfo.label}
								{/if}
								<span class="text-tertiary">{getStepProgress(rootJob, modules.length)}</span>
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
									{@render collapsibleButton(`flow-${flowId}-input`, true)}

									<div class="grow min-w-0 leading-tight">
										<!-- svelte-ignore a11y_no_static_element_interactions -->
										<!-- svelte-ignore a11y_click_events_have_key_events -->
										<div
											class={twMerge(
												'py-1 flex items-center justify-between pr-2 cursor-pointer',
												isCurrent(`flow-${flowId}-input`) ? 'bg-surface-hover' : ''
											)}
											onclick={() => toggleExpanded(`flow-${flowId}-input`)}
											data-nav-id={`flow-${flowId}-input`}
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

							{#if modules.length > 0}
								{#each modules as module (module.id)}
									{@const isLeafStep = !hasSubflows(module)}
									{@const status = localModuleStates[module.id]?.type}
									{@const isRunning = status === 'InProgress' || status === 'WaitingForExecutor'}
									{@const hasEmptySubflowValue = hasEmptySubflow(module.id, module.value.type)}
									{@const isCollapsible = !hasEmptySubflowValue}
									<li class="border-b flex flex-row">
										{@render collapsibleButton(module.id, isCollapsible, isRunning)}
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
														: '',
													isCurrent(module.id) ? 'bg-surface-hover' : ''
												)}
												onclick={isCollapsible ? () => toggleExpanded(module.id) : undefined}
												data-nav-id={module.id}
											>
												<div class="flex items-center gap-2 grow min-w-0">
													<!-- Step icon -->
													{@render stepIcon(
														module.value.type,
														status as FlowStatusModule['type'],
														flowInfo.parentsWithErrors.has(module.id)
													)}

													<div class="flex items-center gap-2">
														<span class="text-xs font-mono">
															<b>
																{mode === 'aiagent'
																	? module.summary
																		? 'Tool call'
																		: 'Message'
																	: module.id}
															</b>
															{#if mode === 'flow'}
																{#if module.value.type === 'forloopflow'}
																	For loop
																{:else if module.value.type === 'whileloopflow'}
																	While loop
																{:else if module.value.type === 'branchall'}
																	Branch to all
																{:else if module.value.type === 'branchone'}
																	Branch to one
																{:else if module.value.type === 'flow'}
																	Subflow
																{:else}
																	Step
																{/if}
															{/if}
															{#if module.summary}
																: {module.summary}
															{/if}
															{#if hasEmptySubflowValue}
																<span class="text-tertiary">
																	{#if module.value.type === 'forloopflow' || module.value.type === 'whileloopflow'}
																		(empty loop)
																	{:else if module.value.type === 'branchall' || module.value.type === 'branchone'}
																		(no branch)
																	{/if}
																</span>
															{/if}
														</span>
														{#if !hasEmptySubflowValue && localModuleStates[module.id]?.flow_jobs && (module.value.type === 'forloopflow' || module.value.type === 'whileloopflow')}
															<span
																class="text-xs font-mono font-medium inline-flex items-center grow min-w-0 -my-2"
															>
																<span onclick={(e) => e.stopPropagation()}>
																	<FlowJobsMenu
																		moduleId={module.id}
																		id={module.id}
																		{onSelectedIteration}
																		flowJobsSuccess={localModuleStates[module.id]
																			?.flow_jobs_success}
																		flowJobs={localModuleStates[module.id]?.flow_jobs}
																		selected={localModuleStates[module.id]?.selectedForloopIndex ??
																			0}
																		selectedManually={localModuleStates[module.id]
																			?.selectedForLoopSetManually ?? false}
																		showIcon={false}
																	/>
																</span>
																{#if module.value.type === 'forloopflow'}
																	{`/${localModuleStates[module.id]?.iteration_total ?? 0}`}
																{/if}
															</span>
														{/if}
													</div>
												</div>

												{#if isLeafStep}
													{@const jobId = localModuleStates[module.id]?.job_id}
													{#if jobId}
														<a
															href={getJobLink(jobId ?? '')}
															class="text-xs text-primary hover:underline font-mono"
															target="_blank"
															rel="noopener noreferrer"
														>
															{truncateRev(jobId ?? '', 6)}
														</a>
													{/if}
												{/if}
											</div>

											{#if isCollapsible && isExpanded(module.id, isRunning)}
												{@const args = localModuleStates[module.id]?.args}
												{@const logs = localModuleStates[module.id]?.logs}
												{@const result = localModuleStates[module.id]?.result}
												{@const jobId = localModuleStates[module.id]?.job_id}
												<div class="my-1 transition-all duration-200 ease-in-out">
													<!-- Show child steps if they exist -->
													{#each getSubflows(module) as subflow}
														{@const subflowJob = {
															id: jobId,
															type:
																localModuleStates[module.id]?.type === 'Failure' ||
																localModuleStates[module.id]?.type === 'Success'
																	? 'CompletedJob'
																	: ('QueuedJob' as Job['type']),
															logs,
															result,
															args,
															success: localModuleStates[module.id]?.type === 'Success'
														}}
														<div class="border-l mb-2">
															<!-- Recursively render child steps using FlowLogViewer -->
															<FlowLogViewer
																modules={subflow.modules}
																{localModuleStates}
																rootJob={subflowJob}
																flowStatus={localModuleStates[module.id]?.type}
																{expandedRows}
																{allExpanded}
																{showResultsInputs}
																{toggleExpanded}
																toggleExpandAll={undefined}
																{workspaceId}
																{render}
																level={level + 1}
																flowId={subflow.flowId}
																flowSummary={subflow.label}
																{onSelectedIteration}
																{getSelectedIteration}
																{currentId}
																bind:navigationChain={subloopNavigationChains[subflow.flowId]}
															/>
														</div>
													{/each}
													<!-- Show input arguments -->
													{#if getSubflows(module).length === 0}
														{#if showResultsInputs && isLeafStep && args && Object.keys(args).length > 0}
															<div class="mb-2">
																<!-- svelte-ignore a11y_click_events_have_key_events -->
																<!-- svelte-ignore a11y_no_static_element_interactions -->
																<div
																	class={twMerge(
																		'flex items-center gap-1 cursor-pointer hover:text-primary text-xs font-mono font-medium mb-1',
																		isCurrent(`${module.id}-input`) ? 'bg-surface-hover' : ''
																	)}
																	onclick={() => toggleExpanded(`${module.id}-input`)}
																	data-nav-id={`${module.id}-input`}
																>
																	{#if isExpanded(`${module.id}-input`)}
																		<ChevronDown size={8} />
																	{:else}
																		<ChevronRight size={8} />
																	{/if}
																	Input
																</div>
																{#if isExpanded(`${module.id}-input`)}
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
														{:else if jobId && !hasSubflows(module)}
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
																	class={twMerge(
																		'flex items-center gap-1 cursor-pointer hover:text-primary text-xs font-mono font-medium mb-1',
																		isCurrent(`${module.id}-result`) ? 'bg-surface-hover' : ''
																	)}
																	onclick={() => toggleExpanded(`${module.id}-result`)}
																	data-nav-id={`${module.id}-result`}
																>
																	{#if isExpanded(`${module.id}-result`)}
																		<ChevronDown size={8} />
																	{:else}
																		<ChevronRight size={8} />
																	{/if}
																	Result
																</div>
																{#if isExpanded(`${module.id}-result`)}
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
								<li class="border-b flex">
									{@render collapsibleButton(`flow-${flowId}-result`, true)}
									<div class="w-full leading-tight">
										<!-- svelte-ignore a11y_no_static_element_interactions -->
										<!-- svelte-ignore a11y_click_events_have_key_events -->
										<div
											class={twMerge(
												'py-1 flex items-center justify-between pr-2 cursor-pointer',
												isCurrent(`flow-${flowId}-result`) ? 'bg-surface-hover' : ''
											)}
											onclick={() => toggleExpanded(`flow-${flowId}-result`)}
											data-nav-id={`flow-${flowId}-result`}
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

{#snippet collapsibleButton(id: string, isCollapsible: boolean, isRunning?: boolean)}
	<div class={twMerge('align-top', isCurrent(id) ? 'border-l border-l-gray-400 -ml-[1px]' : '')}>
		{#if isCollapsible}
			<button
				class={twMerge(
					'py-2 leading-tight w-4 flex items-center justify-center text-xs text-tertiary hover:text-primary',
					isCurrent(id) ? 'bg-surface-hover ' : ''
				)}
				onclick={() => toggleExpanded(id)}
			>
				{#if isExpanded(id, isRunning)}
					<ChevronDown size={8} strokeWidth={isCurrent(id) ? 4 : 2} />
				{:else}
					<ChevronRight size={8} strokeWidth={isCurrent(id) ? 4 : 2} />
				{/if}
			</button>
		{:else}
			<!-- Empty subflow - no collapse button, just spacing -->
			<div class="w-4"></div>
		{/if}
	</div>
{/snippet}

<style>
	.transition-all {
		transition: all 0.2s ease-in-out;
	}
</style>
