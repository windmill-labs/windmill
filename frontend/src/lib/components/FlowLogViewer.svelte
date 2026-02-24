<script lang="ts">
	import {
		GitBranch,
		Repeat,
		Code,
		ArrowDownToLine,
		ArrowDownFromLine,
		FoldVertical,
		UnfoldVertical,
		ExternalLink,
		Keyboard
	} from 'lucide-svelte'
	import { base } from '$lib/base'
	import { workspaceStore } from '$lib/stores'
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
	import FlowLogRow from './FlowLogRow.svelte'
	import { Tooltip } from './meltComponents'
	import FlowTimelineBar from './FlowTimelineBar.svelte'
	import { getActiveReplay } from './recording/flowRecording.svelte'

	type RootJobData = Partial<Job>

	interface Props {
		modules: FlowModule[]
		localModuleStates: Record<string, GraphModuleState>
		rootJob: RootJobData | undefined
		expandedRows: Record<string, boolean>
		allExpanded?: boolean
		showResultsInputs?: boolean
		toggleExpanded: (id: string) => void
		toggleExpandAll?: () => void
		workspaceId: string | undefined
		render: boolean
		level?: number
		flowId: string
		onSelectedIteration?: (
			detail:
				| { id: string; index: number; manuallySet: true; moduleId: string }
				| { manuallySet: false; moduleId: string }
		) => Promise<void>
		getSelectedIteration: (stepId: string) => number
		flowSummary?: string
		mode?: 'flow' | 'aiagent'
		currentId?: string | null
		navigationChain?: NavigationChain
		select: (id: string) => void
		timelineMin?: number
		timelineTotal?: number
		timelineItems?: Record<
			string,
			Array<{ created_at?: number; started_at?: number; duration_ms?: number; id: string }>
		>
		timelineNow: number
		timelineAvailableWidths: Record<string, number>
		timelinelWidth: number
		showTimeline?: boolean
	}

	let {
		modules,
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
		flowSummary,
		mode = 'flow',
		currentId,
		navigationChain = $bindable(),
		select,
		timelineMin: timelineMinAbsolute,
		timelineTotal: timelineTotalAbsolute,
		timelineItems,
		timelineNow,
		timelineAvailableWidths = $bindable(),
		timelinelWidth,
		showTimeline = true
	}: Props = $props()

	let isReplay = $derived(!!getActiveReplay())

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

	function getFlowStatus(job: RootJobData | undefined): FlowStatusModule['type'] | undefined {
		if (!job) return undefined
		if (job.type === 'CompletedJob') {
			return job.success ? 'Success' : 'Failure'
		} else if (job.type === 'QueuedJob') {
			return 'InProgress'
		} else {
			return undefined
		}
	}

	function getStepProgress(job: RootJobData | undefined, totalSteps: number): string {
		if (!job || totalSteps === 0) return ''

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
		if (!rootJob) return undefined
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

	let useRelativeTimeline = $state(false)

	function buildNavigationLinks(): NavigationChain {
		const items: string[] = []

		// Flow header (always first)
		items.push(`flow-${flowId}`)

		if (level > 0 && !isExpanded(`flow-${flowId}`)) {
			return { [`flow-${flowId}`]: { upId: null, downId: null } }
		}

		// Flow input (if exists and shown)
		if (
			showResultsInputs &&
			flowInfo &&
			flowInfo.inputs &&
			Object.keys(flowInfo.inputs).length > 0
		) {
			items.push(`flow-${flowId}-input`)
		}

		// Add modules in order
		modules.forEach((module) => {
			items.push(module.id)

			// If module is expanded, add its children
			if (
				isExpanded(
					module.id,
					localModuleStates[module.id]?.type === 'InProgress' ||
						localModuleStates[module.id]?.type === 'WaitingForExecutor'
				)
			) {
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

					items.push(`${module.id}-logs`)

					// Add result if exists and shown (leaf nodes only)
					if (showResultsInputs && localModuleStates[module.id]?.result !== undefined) {
						items.push(`${module.id}-result`)
					}
				}
			}
		})

		// Flow result (if exists and shown)
		if (
			showResultsInputs &&
			flowInfo &&
			flowInfo.result !== undefined &&
			rootJob?.type === 'CompletedJob'
		) {
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
				// Only scroll if the element is visible (not in a hidden tab)
				const rect = element.getBoundingClientRect()
				const isElementVisible = rect.width > 0 && rect.height > 0

				if (isElementVisible) {
					;(element as HTMLElement).focus?.({ preventScroll: true })
					element.scrollIntoView({ behavior: 'smooth', block: 'nearest' })
				}
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
			// Add default branch for branchone
			if (module.value.type === 'branchone') {
				subflows.push({
					modules: module.value.default,
					label: 'default',
					flowId: `${module.id}-subflow-default`
				})
			}

			// Add all branches
			for (let i = 0; i < module.value.branches.length; i++) {
				const branch = module.value.branches[i]
				subflows.push({
					modules: branch.modules,
					label: branch.summary || `branch ${i + 1}`,
					flowId: `${module.id}-subflow-${i}`
				})
			}
		}

		return subflows
	}

	const { timelineMin, timelineTotal } = $derived({
		timelineMin:
			useRelativeTimeline && rootJob?.started_at
				? new Date(rootJob.started_at).getTime()
				: timelineMinAbsolute,
		timelineTotal:
			useRelativeTimeline && rootJob?.['duration_ms']
				? rootJob['duration_ms']
				: timelineTotalAbsolute
	})

	function getSubflowJob(
		moduleId: string,
		idx: number,
		branchChosen: number | undefined,
		moduleType: FlowModuleValue['type']
	) {
		// if a branch is chosen, ignore the other branches
		if (branchChosen !== undefined && branchChosen !== idx) {
			return undefined
		}

		const jobType =
			localModuleStates[moduleId]?.type === 'Failure' ||
			localModuleStates[moduleId]?.type === 'Success'
				? 'CompletedJob'
				: ('QueuedJob' as Job['type'])
		let jobId = localModuleStates[moduleId]?.job_id
		let timelineItem = timelineItems?.[moduleId]?.find((item) => item.id === jobId)
		let success = localModuleStates[moduleId]?.type === 'Success'
		let result = localModuleStates[moduleId]?.result

		// if the subflow is part of a loop or branchAll
		if (localModuleStates[moduleId]?.flow_jobs) {
			const index =
				moduleType === 'forloopflow' || moduleType === 'whileloopflow'
					? (localModuleStates[moduleId]?.selectedForloopIndex ?? idx)
					: idx
			jobId = localModuleStates[moduleId]?.flow_jobs[index]
			timelineItem = timelineItems?.[moduleId]?.find((item) => item.id === jobId)
			result = localModuleStates[moduleId]?.flow_jobs_results?.[index]
			success = localModuleStates[moduleId]?.flow_jobs_success?.[index] ?? false
		}
		return {
			id: jobId,
			type: jobType,
			logs: localModuleStates[moduleId]?.logs,
			result: result,
			args: localModuleStates[moduleId]?.args,
			success: success,
			started_at: timelineItem?.started_at,
			created_at: timelineItem?.created_at,
			duration_ms: timelineItem?.duration_ms
		} as RootJobData
	}

	function getSelectedIndex(
		moduleId: string,
		moduleItems:
			| Array<{ created_at?: number; started_at?: number; duration_ms?: number; id: string }>
			| undefined
	) {
		if (!moduleItems || !localModuleStates[moduleId]) return undefined

		const idToFind =
			localModuleStates[moduleId].selectedForloop ?? localModuleStates[moduleId].job_id
		const index = moduleItems?.findIndex((item) => item.id === idToFind)
		if (index === -1) {
			return undefined
		}
		return index
	}

	function isJobFailure(jobId?: string, moduleId?: string) {
		if (!moduleId) {
			return rootJob?.type === 'CompletedJob' && rootJob?.['success'] === false
		}
		// if a jobId is provided, check the flow_jobs_success array for a specific job
		if (localModuleStates[moduleId]?.flow_jobs_success && !!jobId) {
			const index = localModuleStates[moduleId]?.flow_jobs?.indexOf(jobId)
			if (index !== undefined && index >= 0) {
				return localModuleStates[moduleId]?.flow_jobs_success?.[index] === false
			}
		}
		return localModuleStates[moduleId]?.type === 'Failure'
	}
</script>

{#if render}
	{#if level === 0 && toggleExpandAll}
		<div class="flex justify-end gap-4 items-center p-2 bg-surface-secondary border-b">
			<Tooltip>
				{#snippet text()}
					Keyboard navigation available - ↑/↓/Enter
				{/snippet}
				<Keyboard size={16} class="text-primary" />
			</Tooltip>
			<div class="flex items-center gap-2 whitespace-nowrap">
				<label for="showTimeline" class="text-xs text-primary hover:text-primary transition-colors"
					>Show timeline</label
				>
				<div class="flex-shrink-0">
					<input
						type="checkbox"
						name="showTimeline"
						id="showTimeline"
						bind:checked={showTimeline}
						class="w-3 h-4 accent-primary -my-1"
					/>
				</div>
			</div>
			<div class="flex items-center gap-2 whitespace-nowrap">
				<label
					for="showResultsInputs"
					class="text-xs text-primary hover:text-primary transition-colors"
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
				class="text-xs text-primary hover:text-primary transition-colors flex items-center gap-2 min-w-24 justify-end"
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
		<FlowLogRow
			id={`flow-${flowId}`}
			isCollapsible={level > 0}
			isRunning={rootJob?.type === 'QueuedJob'}
			{isCurrent}
			{isExpanded}
			{toggleExpanded}
			class={rootJob?.type === undefined ? 'opacity-50' : ''}
			{select}
		>
			{#snippet label()}
				<div class="flex items-center gap-2">
					<!-- Flow icon -->
					{@render flowIcon(getFlowStatus(rootJob), flowInfo?.hasErrors)}

					<div class="text-xs text-left font-mono">
						{mode === 'aiagent' ? 'AI Agent' : level == 0 ? 'Flow' : 'Subflow'}
						{#if flowInfo?.label}
							: {flowInfo.label}
						{/if}
						<span class="text-primary">{getStepProgress(rootJob, modules.length)}</span>
					</div>

					<div
						class="min-w-min grow group"
						bind:clientWidth={
							() => timelineAvailableWidths[flowId] ?? 0,
							(v) => (timelineAvailableWidths[flowId] = v)
						}
					>
						{#if timelineItems && showTimeline && timelineMin != undefined && timelineTotal}
							{@const moduleItems = [
								{
									started_at: rootJob?.started_at
										? new Date(rootJob.started_at).getTime()
										: undefined,
									duration_ms: rootJob?.['duration_ms'] ?? timelineTotal,
									id: flowId
								}
							]}
							<FlowTimelineBar
								total={timelineTotal}
								min={timelineMin}
								items={moduleItems}
								now={timelineNow}
								{timelinelWidth}
								showZoomButtons={level > 0 && isExpanded(`flow-${flowId}`)}
								onZoom={() => {
									useRelativeTimeline = !useRelativeTimeline
								}}
								zoom={useRelativeTimeline ? 'in' : 'out'}
								isJobFailure={(id) => isJobFailure(id)}
							/>
						{/if}
					</div>

					{#if flowInfo?.jobId && !isReplay}
						<a
							href={getJobLink(flowInfo.jobId)}
							class="text-xs text-gray-400 hover:text-primary pl-1"
							target="_blank"
							rel="noopener noreferrer"
							onclick={(e) => e.stopPropagation()}
						>
							<ExternalLink size={12} />
						</a>
					{/if}
				</div>
			{/snippet}

			{#if level === 0 || isExpanded(`flow-${flowId}`, rootJob?.type === 'QueuedJob')}
				<div class="mb-2 transition-all duration-200 ease-in-out w-full">
					<!-- Flow logs -->
					{#if flowInfo?.logs}
						<LogViewer
							content={flowInfo.logs}
							jobId={isReplay ? undefined : flowInfo.jobId}
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
						{#if showResultsInputs && flowInfo?.inputs && Object.keys(flowInfo.inputs).length > 0}
							<FlowLogRow
								id={`flow-${flowId}-input`}
								isCollapsible={true}
								isRunning={false}
								{isCurrent}
								{isExpanded}
								{toggleExpanded}
								{select}
							>
								{#snippet label()}
									<div class="flex items-center gap-2 grow min-w-0">
										<ArrowDownToLine size={10} />
										<span class="text-xs font-mono">Inputs</span>
									</div>
								{/snippet}
								{#if isExpanded(`flow-${flowId}-input`)}
									<div class="my-1 transition-all duration-200 ease-in-out">
										<div class="pl-4">
											<ObjectViewer json={flowInfo.inputs} pureViewer={true} />
										</div>
									</div>
								{/if}
							</FlowLogRow>
						{/if}

						{#if modules.length > 0}
							{#each modules as module (module.id)}
								{@const isLeafStep = !hasSubflows(module)}
								{@const status = localModuleStates[module.id]?.type}
								{@const isRunning = status === 'InProgress' || status === 'WaitingForExecutor'}
								{@const hasEmptySubflowValue = hasEmptySubflow(module.id, module.value.type)}
								{@const isCollapsible = !hasEmptySubflowValue}
								{@const jobId = localModuleStates[module.id]?.job_id}
								{@const moduleItems = timelineItems?.[module.id]}
								{@const branchChosen =
									module.value.type === 'branchone'
										? (localModuleStates[module.id]?.branchChosen ?? 0)
										: undefined}
								<FlowLogRow
									id={module.id}
									{isCollapsible}
									{isRunning}
									{isCurrent}
									{isExpanded}
									{toggleExpanded}
									{select}
								>
									{#snippet label()}
										<div
											class={twMerge(
												'flex items-center justify-between',
												status === 'WaitingForPriorSteps' ||
													status === 'WaitingForEvents' ||
													status === 'WaitingForExecutor' ||
													status === undefined
													? 'opacity-50'
													: ''
											)}
										>
											<div class="flex items-center gap-2">
												<!-- Step icon -->
												{@render stepIcon(
													module.value.type,
													status as FlowStatusModule['type'],
													flowInfo?.parentsWithErrors.has(module.id)
												)}

												<div class="flex items-center gap-2">
													<span class="text-xs font-mono text-left">
														<b class="flex items-center gap-1">
															{#if mode === 'aiagent'}
																{#if module.summary}
																	Tool call: {module.summary}
																{:else}
																	Message
																{/if}
															{:else}
																{module.id}
															{/if}
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
														{#if module.summary && mode !== 'aiagent'}
															: {module.summary}
														{/if}
														{#if hasEmptySubflowValue}
															<span class="text-primary">
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
															class="text-xs font-mono font-medium inline-flex items-center -my-2"
														>
															<button onclick={(e) => e.stopPropagation()}>
																<FlowJobsMenu
																	moduleId={module.id}
																	id={module.id}
																	{onSelectedIteration}
																	flowJobsSuccess={localModuleStates[module.id]?.flow_jobs_success}
																	flowJobs={localModuleStates[module.id]?.flow_jobs}
																	selected={localModuleStates[module.id]?.selectedForloopIndex ?? 0}
																	selectedManually={localModuleStates[module.id]
																		?.selectedForLoopSetManually ?? false}
																	showIcon={false}
																/>
															</button>
															{#if module.value.type === 'forloopflow'}
																{`/${localModuleStates[module.id]?.iteration_total ?? 0}`}
															{/if}
														</span>
													{/if}
												</div>
											</div>

											<div
												class="min-w-min grow {isLeafStep ? 'mr-2' : 'mr-6'} min-h-2"
												bind:clientWidth={
													() => timelineAvailableWidths[module.id] ?? 0,
													(v) => (timelineAvailableWidths[module.id] = v)
												}
											>
												{#if timelineMin != undefined && timelineTotal && moduleItems && showTimeline}
													<FlowTimelineBar
														total={timelineTotal}
														min={timelineMin}
														items={moduleItems ?? []}
														now={timelineNow}
														{timelinelWidth}
														onSelectIteration={(id) => {
															if (
																module.value.type !== 'forloopflow' &&
																module.value.type !== 'whileloopflow'
															) {
																return
															}
															const index =
																localModuleStates[module.id]?.flow_jobs?.indexOf(id) ?? undefined
															if (index !== undefined) {
																onSelectedIteration?.({
																	id,
																	index,
																	manuallySet: true,
																	moduleId: module.id
																})
															}
														}}
														showIterations={localModuleStates[module.id]?.flow_jobs}
														selectedIndex={getSelectedIndex(module.id, moduleItems)}
														idToIterationIndex={(id) => {
															return localModuleStates[module.id]?.flow_jobs?.indexOf(id)
														}}
														isJobFailure={() => isJobFailure(undefined, module.id)}
													/>
												{/if}
											</div>

											{#if isLeafStep && jobId && !isReplay}
												<a
													href={getJobLink(jobId ?? '')}
													class="text-xs text-gray-400 hover:text-primary pl-1"
													target="_blank"
													rel="noopener noreferrer"
												>
													<ExternalLink size={12} />
												</a>
											{/if}
										</div>
									{/snippet}

									{#if isCollapsible && isExpanded(module.id, isRunning)}
										{@const subflows = getSubflows(module)}
										<div class="my-1 transition-all duration-200 ease-in-out border-l">
											<!-- Show child steps if they exist -->
											{#each subflows as subflow, idx}
												{@const subflowJob = getSubflowJob(
													module.id,
													idx,
													branchChosen,
													module.value.type
												)}
												<div class="border-l mb-2">
													<!-- Recursively render child steps using FlowLogViewer -->
													<FlowLogViewer
														modules={subflow.modules}
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
														flowId={subflow.flowId}
														flowSummary={subflow.label}
														{onSelectedIteration}
														{getSelectedIteration}
														{currentId}
														bind:navigationChain={subloopNavigationChains[subflow.flowId]}
														{select}
														{timelineNow}
														{timelineMin}
														{timelineTotal}
														{timelineItems}
														bind:timelineAvailableWidths
														{timelinelWidth}
														{showTimeline}
													/>
												</div>
											{/each}

											{#if subflows.length === 0}
												{@const args = localModuleStates[module.id]?.args}
												{@const logs = localModuleStates[module.id]?.logs}
												{@const result = localModuleStates[module.id]?.result}
												{@const jobId = localModuleStates[module.id]?.job_id}
												<!-- Show input arguments -->
												{#if showResultsInputs && isLeafStep && args && Object.keys(args).length > 0}
													<FlowLogRow
														id={`${module.id}-input`}
														isCollapsible={true}
														isRunning={false}
														{isCurrent}
														{isExpanded}
														{toggleExpanded}
														{select}
													>
														{#snippet label()}
															<div class="flex items-center gap-2 grow min-w-0">
																<ArrowDownFromLine size={10} />
																<span class="text-xs font-mono">Input</span>
															</div>
														{/snippet}
														{#if isExpanded(`${module.id}-input`)}
															<div class="pl-4">
																<ObjectViewer json={args} pureViewer={true} />
															</div>
														{/if}
													</FlowLogRow>
												{/if}

												<!-- Show logs if they exist -->
												<!-- svelte-ignore a11y_click_events_have_key_events -->
												<!-- svelte-ignore a11y_no_static_element_interactions -->
												{#if logs}
													<div onclick={() => select(`${module.id}-logs`)}>
														<LogViewer
															content={logs}
															jobId={isReplay ? undefined : (jobId ?? '')}
															isLoading={false}
															small={true}
															download={false}
															noAutoScroll={true}
															tag={undefined}
															noPadding
															wrapperClass={twMerge(
																'w-full mb-2 px-2',
																isCurrent(`${module.id}-logs`)
																	? 'border-l-2 border-l-gray-400 -ml-[2px]'
																	: ''
															)}
															navigationId={`${module.id}-logs`}
														/>
													</div>
												{:else if jobId && !hasSubflows(module)}
													<div
														class={twMerge(
															'w-full mb-2 px-2',
															isCurrent(`${module.id}-logs`)
																? 'border-l-2 border-l-gray-400 -ml-[2px]'
																: ''
														)}
													>
														<div class="text-xs text-primary font-mono"> No logs available </div>
													</div>
												{/if}

												<!-- Show result if completed -->
												{#if showResultsInputs && isLeafStep && result !== undefined && (status === 'Success' || status === 'Failure')}
													<FlowLogRow
														id={`${module.id}-result`}
														isCollapsible={true}
														isRunning={false}
														{isCurrent}
														{isExpanded}
														{toggleExpanded}
														{select}
													>
														{#snippet label()}
															<div class="flex items-center gap-2 grow min-w-0">
																<ArrowDownFromLine size={10} />
																<span class="text-xs font-mono">Result</span>
															</div>
														{/snippet}
														{#if isExpanded(`${module.id}-result`)}
															<div class="pl-4">
																<ObjectViewer json={result} pureViewer={true} />
															</div>
														{/if}
													</FlowLogRow>
												{/if}
											{/if}
										</div>
									{/if}
								</FlowLogRow>
							{/each}
						{/if}

						<!-- Flow result as last row entry -->
						{#if showResultsInputs && flowInfo?.result !== undefined && rootJob?.type === 'CompletedJob'}
							<FlowLogRow
								id={`flow-${flowId}-result`}
								isCollapsible={true}
								isRunning={false}
								{isCurrent}
								{isExpanded}
								{toggleExpanded}
								{select}
							>
								{#snippet label()}
									<div class="flex items-center gap-2 grow min-w-0">
										<ArrowDownFromLine size={10} />
										<span class="text-xs font-mono">Results</span>
									</div>
								{/snippet}
								<ObjectViewer json={flowInfo.result} pureViewer={true} />
							</FlowLogRow>
						{/if}
					</ul>
				</div>
			{/if}
		</FlowLogRow>
	</ul>
{/if}

{#snippet flowIcon(status: FlowStatusModule['type'] | undefined, hasErrors: boolean | undefined)}
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
	hasErrors: boolean | undefined
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
