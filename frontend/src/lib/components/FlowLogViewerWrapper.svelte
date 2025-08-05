<script lang="ts">
	import type { FlowStatusModule, Job } from '$lib/gen'
	import type { Writable } from 'svelte/store'
	import type { GraphModuleState } from './graph'
	import { JobService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import FlowLogViewer from './FlowLogViewer.svelte'
	import FlowLogsLoader from './FlowLogsLoader.svelte'
	import type { FlowData, StepData } from './FlowLogUtils'
	import { untrack } from 'svelte'

	interface Props {
		innerModules: FlowStatusModule[]
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
		innerModules,
		job,
		localModuleStates,
		workspaceId,
		render,
		refreshLog = false,
		onSelectedIteration
	}: Props = $props()

	// Cache for fetched subflow jobs
	let subflowJobs: Map<string, Job> = $state(new Map())

	// State for tracking expanded rows
	let expandedRows: Set<string> = $state(new Set())

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

	// Build the flow data structure
	async function buildFlowData(modules: FlowStatusModule[], rootJob: Job): Promise<FlowData> {
		const steps: StepData[] = []

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
				iterationTotal: state?.iteration_total ?? 0
			}

			// Handle subflows (branchall, brancheone, forloopflow, whileloopflow)
			if (stepData.type === 'branchone' && stepData.jobId) {
				stepData.subflows = []
				const subflowJob = await fetchSubflowJob(stepData.jobId)
				if (subflowJob) {
					const subflowData = await buildFlowData(subflowJob.flow_status?.modules || [], subflowJob)
					stepData.subflows.push(subflowData)
				}
			} else if (
				['branchall', 'branchone', 'forloopflow', 'whileloopflow'].includes(stepData.type ?? '') &&
				module.flow_jobs &&
				module.flow_jobs.length > 0
			) {
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

			steps.push(stepData)
		}

		return {
			jobId: rootJob.id,
			inputs: rootJob.args || {},
			result: rootJob.type === 'CompletedJob' ? rootJob.result : undefined,
			success: rootJob.type === 'CompletedJob' ? rootJob.success : undefined,
			logs: rootJob.logs || '',
			steps,
			status: rootJob.type
		}
	}

	// Build the flow data when dependencies change
	let flowData: FlowData | null = $state(null)

	$effect(() => {
		if (render && innerModules.length > 0 && $localModuleStates) {
			untrack(() => buildFlowData(innerModules, job)).then((data) => {
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

	function getSelectedIteration(stepId: string): number {
		return $localModuleStates[stepId]?.selectedForloopIndex ?? 0
	}
</script>

<div class="w-full rounded-md overflow-hidden border">
	{#if flowData}
		<!-- Log polling component -->
		<FlowLogsLoader {expandedRows} {workspaceId} {refreshLog} {localModuleStates} />

		<FlowLogViewer
			{flowData}
			{expandedRows}
			{toggleExpanded}
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
