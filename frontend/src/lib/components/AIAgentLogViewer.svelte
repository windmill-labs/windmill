<script lang="ts">
	import type { GraphModuleState } from './graph'
	import { JobService, type FlowModule, type FlowStatusModule, type Job } from '$lib/gen'
	import { writable, type Writable } from 'svelte/store'
	import { workspaceStore } from '$lib/stores'
	import FlowLogViewerWrapper from './FlowLogViewerWrapper.svelte'

	interface Props {
		aiAgentStatus: {
			jobId: string
			actions: NonNullable<FlowStatusModule['agent_actions']>
			tools: FlowModule[]
		}
		workspaceId?: string | undefined
	}

	let { aiAgentStatus, workspaceId }: Props = $props()

	let localModuleStates: Writable<Record<string, GraphModuleState>> = writable({})
	let job: Job | undefined = $state(undefined)
	async function loadToolJobs() {
		job = await JobService.getJob({
			id: aiAgentStatus.jobId,
			workspace: workspaceId ?? $workspaceStore!
		})
		job = {
			...job,
			raw_flow: {
				modules: aiAgentStatus.actions
					.map((toolCall, idx) => {
						if (toolCall.type === 'message') {
							return {
								id: idx.toString(),
								value: {
									type: 'identity' as const
								}
							}
						} else {
							const module = aiAgentStatus.tools.find((m) => m.summary === toolCall.function_name)
							return module
								? {
										...module,
										id: idx.toString()
									}
								: undefined
						}
					})
					.filter((m) => m !== undefined)
			}
		}
		const promises = aiAgentStatus.actions.map(async (toolCall, idx) => {
			if (toolCall.type === 'tool_call') {
				const job = await JobService.getJob({
					id: toolCall.job_id,
					workspace: workspaceId ?? $workspaceStore!
				})
				let started_at = job.started_at ? new Date(job.started_at).getTime() : undefined
				$localModuleStates[idx.toString()] = {
					args: job.args,
					type: job['success'] ? 'Success' : 'Failure',
					logs: job.logs,
					result: job['result'],
					job_id: toolCall.job_id,
					tag: job.tag,
					duration_ms: job['duration_ms'],
					started_at: started_at
				}
			} else {
				$localModuleStates[idx.toString()] = {
					type: 'Success',
					args: {},
					logs: '',
					result: toolCall.content
				}
			}
		})

		await Promise.all(promises)
	}

	$effect(() => {
		loadToolJobs()
	})
</script>

{#if job}
	<div class="p-2">
		<FlowLogViewerWrapper
			{job}
			{localModuleStates}
			{workspaceId}
			render={true}
			onSelectedIteration={async () => {}}
			mode="aiagent"
		/>
	</div>
{/if}
