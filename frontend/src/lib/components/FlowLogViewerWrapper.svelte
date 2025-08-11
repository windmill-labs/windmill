<script lang="ts">
	import type { BranchAll, FlowModuleValue, Job } from '$lib/gen'
	import type { Writable } from 'svelte/store'
	import type { GraphModuleState } from './graph'
	import { JobService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import FlowLogViewer from './FlowLogViewer.svelte'
	import FlowLogsLoader from './FlowLogsLoader.svelte'
	import type { FlowData, StepData } from './FlowLogUtils'
	import { untrack } from 'svelte'

	interface Props {
		job: Job
		localModuleStates: Writable<Record<string, GraphModuleState>>
		workspaceId: string | undefined
		render: boolean
		refreshLog?: boolean
		onSelectedIteration: (
			detail:
				| { id: string; index: number; manuallySet: true; moduleId: string }
				| { manuallySet: false; moduleId: string }
		) => Promise<void>
	}

	let {
		job,
		localModuleStates,
		workspaceId,
		render,
		refreshLog = false,
		onSelectedIteration
	}: Props = $props()

	// Cache for fetched subflow jobs
	let subflowJobs: Map<string, Job> = $state(new Map())

	// State for tracking expanded rows - using Record to allow explicit control
	let expandedRows: Record<string, boolean> = $state({})
	let allExpanded = $state(false)
	let showResultsInputs = $state(true)

	// Fetch subflow job data
	async function fetchSubflowJob(jobId: string): Promise<Job | null> {
		if (!jobId) return null
		if (subflowJobs.has(jobId)) {
			const job = subflowJobs.get(jobId)
			if (job?.type === 'CompletedJob') {
				//use cache only  if the job in cache is completed
				return job
			}
		}

		try {
			const jobData = await JobService.getJob({
				workspace: workspaceId ?? $workspaceStore ?? '',
				id: jobId,
				noLogs: true
			})

			subflowJobs.set(jobId, jobData)
			subflowJobs = new Map(subflowJobs)
			return jobData
		} catch (error) {
			console.error('Failed to fetch subflow job:', jobId, error)
			return null
		}
	}

	function getBranchChosenLabel(
		module: FlowModuleValue | undefined,
		branchChosen: number | undefined
	): string | undefined {
		if (!module) return undefined
		if (module.type === 'branchone') {
			if (branchChosen === undefined || branchChosen === 0) return 'Default branch'
			const summary = module.branches?.[branchChosen - 1]?.summary
			return summary && summary !== '' ? `Branch ${summary}` : `Branch ${branchChosen}`
		}
		return undefined
	}

	// Check if a flow contains any errors and set hasErrors appropriately
	// Returns true if any errors exist anywhere in the flow hierarchy
	function checkAndMarkFlowErrors(flowData: FlowData): boolean {
		let hasAnyErrors = false

		// First, check if the flow itself failed
		const flowFailed = flowData.status === 'CompletedJob' && flowData.success === false
		if (flowFailed) {
			hasAnyErrors = true
		}

		// Check each step and its subflows
		for (const step of flowData.steps) {
			let stepHasChildErrors = false

			// Check if step itself failed
			if (step.status === 'Failure') {
				hasAnyErrors = true
				// If step failed, parent flow should show error indicator
				flowData.hasErrors = true
			}

			// Check subflows recursively
			if (step.subflows && step.subflows.length > 0) {
				for (const subflow of step.subflows) {
					if (checkAndMarkFlowErrors(subflow)) {
						hasAnyErrors = true
						stepHasChildErrors = true
					}
				}
			}

			// Set step's hasErrors flag based on child errors only
			step.hasErrors = stepHasChildErrors
		}

		// If flow itself failed, don't show error indicator (parent will show it)
		// If flow didn't fail but has child errors, show error indicator
		if (flowFailed) {
			flowData.hasErrors = false
		} else if (hasAnyErrors) {
			flowData.hasErrors = true
		} else {
			flowData.hasErrors = false
		}

		return hasAnyErrors
	}

	// Build the flow data structure
	async function buildFlowData(rootJob: Job, flowLabel?: string): Promise<FlowData> {
		const steps: StepData[] = []
		const modules = rootJob.flow_status?.modules || []

		for (let i = 0; i < modules.length; i++) {
			const module = modules[i]
			if (!module.id) continue

			const stepNumber = i + 1
			const state = $localModuleStates[module.id]
			const summary = rootJob.raw_flow?.modules?.[i]?.summary

			const stepData: StepData = {
				stepId: module.id,
				stepNumber,
				summary,
				inputs: state?.args || {},
				result: state?.result,
				jobId: state?.job_id || '',
				logs: state?.logs || '',
				status: state?.type,
				type: rootJob.raw_flow?.modules?.[i]?.value?.type,
				selectedIteration: state?.selectedForloopIndex ?? 0,
				iterationTotal: state?.iteration_total ?? 0,
				flowJobs: state?.flow_jobs,
				flowJobsSuccess: state?.flow_jobs_success,
				selectedManually: state?.selectedForLoopSetManually
			}

			// Handle subflows (branchall, brancheone, forloopflow, whileloopflow)
			if (stepData.type === 'branchone' && stepData.jobId) {
				stepData.subflows = []
				const subflowJob = await fetchSubflowJob(stepData.jobId)
				if (subflowJob) {
					const subflowData = await buildFlowData(
						subflowJob,
						getBranchChosenLabel(rootJob.raw_flow?.modules?.[i]?.value, state?.branchChosen)
					)
					stepData.subflows.push(subflowData)
				}
			} else if (
				['branchall', 'forloopflow', 'whileloopflow'].includes(stepData.type ?? '') &&
				module.flow_jobs &&
				module.flow_jobs.length > 0
			) {
				stepData.subflows = []

				for (const [index, subflowJobId] of module.flow_jobs.entries()) {
					const subflowJob = await fetchSubflowJob(subflowJobId)

					// For branch all we can use the branch summary as the flow label
					let flowLabel: string | undefined = undefined
					if (stepData.type === 'branchall') {
						const moduleValue = rootJob.raw_flow?.modules?.[i]?.value as BranchAll
						flowLabel = moduleValue.branches?.[index]?.summary
					}

					if (subflowJob) {
						const subflowData = await buildFlowData(subflowJob, flowLabel)
						stepData.subflows.push(subflowData)
					}
				}
			} else if (
				module.flow_jobs?.length == 0 &&
				module.job == '00000000-0000-0000-0000-000000000000'
			) {
				stepData.emptySubflow = true
			}

			steps.push(stepData)
		}

		const flowData: FlowData = {
			jobId: rootJob.id,
			inputs: rootJob.args || {},
			result: rootJob.type === 'CompletedJob' ? rootJob.result : undefined,
			success: rootJob.type === 'CompletedJob' ? rootJob.success : undefined,
			logs: rootJob.logs || '',
			steps,
			status: rootJob.type,
			label: flowLabel,
			flow_status: 'flow_status' in rootJob ? rootJob.flow_status : undefined
		}

		// Check and mark all errors in one pass
		checkAndMarkFlowErrors(flowData)

		return flowData
	}

	// Build the flow data when dependencies change
	let flowData: FlowData | null = $state(null)

	$effect(() => {
		if (render && job && $localModuleStates) {
			untrack(() => buildFlowData(job)).then((data) => {
				flowData = data
			})
		}
	})

	function toggleExpanded(id: string) {
		// If not in record, use opposite of allExpanded as new state
		// If in record, toggle the current state
		const currentState = expandedRows[id] ?? allExpanded
		expandedRows[id] = !currentState
		expandedRows = { ...expandedRows }
	}

	function getSelectedIteration(stepId: string): number {
		return $localModuleStates[stepId]?.selectedForloopIndex ?? 0
	}

	function toggleExpandAll() {
		allExpanded = !allExpanded
		expandedRows = {}
	}
</script>

<div class="w-full rounded-md overflow-hidden border">
	{#if flowData}
		<!-- Log polling component -->
		<FlowLogsLoader {expandedRows} {allExpanded} {workspaceId} {refreshLog} {localModuleStates} />

		<FlowLogViewer
			{flowData}
			{expandedRows}
			{allExpanded}
			{showResultsInputs}
			{toggleExpanded}
			{toggleExpandAll}
			{onSelectedIteration}
			{workspaceId}
			{render}
			{getSelectedIteration}
			flowId="root"
		/>
	{:else}
		<div class="p-4 text-center text-tertiary">Loading flow data...</div>
	{/if}
</div>
