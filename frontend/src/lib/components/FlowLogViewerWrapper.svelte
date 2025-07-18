<script lang="ts">
	import type { FlowStatusModule, Job } from '$lib/gen'
	import type { Writable } from 'svelte/store'
	import type { GraphModuleState } from './graph'
	import { JobService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
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

	let { innerModules, job, localModuleStates, workspaceId, render }: Props = $props()

	// Cache for fetched logs per job ID
	let jobLogs: Map<string, string> = $state(new Map())

	// Cache for fetched subflow jobs
	let subflowJobs: Map<string, Job> = $state(new Map())

	// State for tracking expanded rows
	let expandedRows: Set<string> = $state(new Set())

	// State for tracking selected iteration for forloop/whileloop steps
	let selectedIterations: Record<string, number> = $state({})

	// Fetch logs for a specific job
	async function fetchJobLogs(jobId: string): Promise<string> {
		if (!jobId) return ''
		if (jobLogs.has(jobId)) return jobLogs.get(jobId)!

		try {
			const jobData = await JobService.getJob({
				workspace: workspaceId ?? $workspaceStore ?? '',
				id: jobId,
				noLogs: false
			})

			const logs = jobData.logs || ''
			jobLogs.set(jobId, logs)
			jobLogs = new Map(jobLogs)
			return logs
		} catch (error) {
			console.error('Failed to fetch logs for job:', jobId, error)
			return ''
		}
	}

	// Fetch subflow job data
	async function fetchSubflowJob(jobId: string): Promise<Job | null> {
		if (!jobId) return null
		if (subflowJobs.has(jobId)) return subflowJobs.get(jobId)!

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

	function getStepStatus(
		module: FlowStatusModule
	): 'success' | 'failure' | 'in_progress' | 'waiting' {
		if (module.type === 'Success') return 'success'
		if (module.type === 'Failure') return 'failure'
		if (module.type === 'InProgress') return 'in_progress'
		return 'waiting'
	}

	function isSubflowStep(stepIndex: number): boolean {
		const stepType = job.raw_flow?.modules?.[stepIndex]?.value?.type
		return stepType
			? ['branchall', 'brancheone', 'forloopflow', 'whileloopflow'].includes(stepType)
			: false
	}

	function isIterationStep(stepIndex: number): boolean {
		const stepType = job.raw_flow?.modules?.[stepIndex]?.value?.type
		return stepType ? ['forloopflow', 'whileloopflow'].includes(stepType) : false
	}

	// Build the flow data structure
	async function buildFlowData(modules: FlowStatusModule[], rootJob: Job): Promise<FlowData> {
		const steps: StepData[] = []

		for (let i = 0; i < modules.length; i++) {
			const module = modules[i]
			if (!module.id) continue

			const stepNumber = i + 1
			const status = getStepStatus(module)
			const state = $localModuleStates[module.id]
			const summary = rootJob.raw_flow?.modules?.[i]?.summary
			const isSubflow = isSubflowStep(i)
			const isIteration = isIterationStep(i)

			let logs = ''
			if (module.job) {
				logs = await fetchJobLogs(module.job)
			}

			const stepData: StepData = {
				stepId: module.id,
				stepNumber,
				summary,
				inputs: state?.args || {},
				result: state?.result,
				jobId: module.job,
				logs,
				status
			}

			// Handle subflows (branchall, brancheone, forloopflow, whileloopflow)
			if (isSubflow && module.flow_jobs && module.flow_jobs.length > 0) {
				if (isIteration) {
					// For forloop/whileloop, each flow_job is an iteration
					stepData.iterations = []
					stepData.selectedIteration = selectedIterations[module.id] ?? 0

					for (let j = 0; j < module.flow_jobs.length; j++) {
						const iterationJobId = module.flow_jobs[j]
						const iterationJob = await fetchSubflowJob(iterationJobId)
						if (iterationJob) {
							const iterationFlow = await buildFlowData(
								iterationJob.flow_status?.modules || [],
								iterationJob
							)
							stepData.iterations.push(iterationFlow)
						}
					}
				} else {
					// For branchall/brancheone, each flow_job is a separate subflow
					stepData.subflows = []
					for (const subflowJobId of module.flow_jobs) {
						const subflowJob = await fetchSubflowJob(subflowJobId)
						if (subflowJob) {
							const subflowData = await buildFlowData(
								subflowJob.flow_status?.modules || [],
								subflowJob
							)
							stepData.subflows.push(subflowData)
						}
					}
				}
			}

			steps.push(stepData)
		}

		return {
			jobId: rootJob.id,
			inputs: rootJob.args || {},
			result: rootJob.type === 'CompletedJob' ? rootJob.result : undefined,
			steps
		}
	}

	// Build the flow data when dependencies change
	let flowData: FlowData | null = $state(null)

	$effect(() => {
		if (render && innerModules.length > 0) {
			buildFlowData(innerModules, job).then((data) => {
				flowData = data
			})
		}
	})

	function toggleExpanded(id: string) {
		if (expandedRows.has(id)) {
			expandedRows.delete(id)
		} else {
			expandedRows.add(id)
		}
		expandedRows = new Set(expandedRows)
	}

	function updateSelectedIteration(stepId: string, iteration: number) {
		selectedIterations[stepId] = iteration
	}
</script>

<div class="w-full rounded-md overflow-hidden border">
	{#if flowData}
		<FlowLogViewer
			{flowData}
			{expandedRows}
			{selectedIterations}
			{toggleExpanded}
			{updateSelectedIteration}
			{workspaceId}
			{render}
		/>
	{:else}
		<div class="p-4 text-center text-tertiary">Loading flow data...</div>
	{/if}
</div>
