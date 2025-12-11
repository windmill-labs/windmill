<script lang="ts">
	import type { GraphModuleState } from './graph'
	import {
		JobService,
		type CompletedJob,
		type FlowModule,
		type FlowStatusModule,
		type Job
	} from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import FlowLogViewerWrapper from './FlowLogViewerWrapper.svelte'
	import { z } from 'zod'
	import { onMount } from 'svelte'
	import type { AgentTool } from './flows/agentToolUtils'

	type AgentActionWithContent = NonNullable<FlowStatusModule['agent_actions']>[number] & {
		content?: unknown
	}

	const resultSchema = z.object({
		messages: z.array(
			z.object({
				role: z.string(),
				content: z.unknown(),
				agent_action: z
					.union([
						z.object({
							type: z.literal('tool_call'),
							job_id: z.string(),
							module_id: z.string(),
							function_name: z.string()
						}),
						z.object({
							type: z.literal('mcp_tool_call'),
							call_id: z.string(),
							function_name: z.string(),
							resource_path: z.string(),
							arguments: z.record(z.any(), z.any()).optional()
						}),
						z.object({
							type: z.literal('message')
						})
					])
					.optional()
			})
		)
	})

	interface Props {
		tools: AgentTool[]
		agentJob: Partial<CompletedJob> & Pick<CompletedJob, 'id'> & { type: 'CompletedJob' }
		workspaceId?: string | undefined
		storedToolCallJobs?: Record<number, Job>
		onToolJobLoaded?: (job: Job, idx: number) => void
	}

	let { tools, agentJob, workspaceId, onToolJobLoaded, storedToolCallJobs }: Props = $props()

	const fakeModuleStates: Record<string, GraphModuleState> = $state({})

	async function loadMissingJobs(agentActions: AgentActionWithContent[]) {
		const promises = agentActions.map(async (toolCall, idx) => {
			if (toolCall.type === 'tool_call') {
				let job: Job | undefined = storedToolCallJobs?.[idx]

				if (!job || job.type !== 'CompletedJob') {
					job = await JobService.getJob({
						id: toolCall.job_id,
						workspace: workspaceId ?? $workspaceStore!
					})
				}
				fakeModuleStates[idx.toString()] = {
					args: job.args,
					type: job['success'] ? 'Success' : 'Failure',
					logs: job.logs,
					result: job['result'],
					job_id: toolCall.job_id
				}
				onToolJobLoaded?.(job, idx)
			} else if (toolCall.type === 'mcp_tool_call') {
				fakeModuleStates[idx.toString()] = {
					type: 'Success',
					args: toolCall.arguments ?? {},
					logs: '',
					result: toolCall.content
				}
			} else {
				fakeModuleStates[idx.toString()] = {
					type: 'Success',
					args: {},
					logs: '',
					result: toolCall.content
				}
			}
		})

		await Promise.all(promises)
	}

	let job: Partial<Job> | undefined = $state(undefined)
	async function loadToolCalls() {
		let parsedResult = resultSchema.safeParse(agentJob.result)
		if (!parsedResult.success) {
			console.error('Invalid result', parsedResult.error)
			return
		}
		let agentActions = parsedResult.data.messages
			.map(
				(m) =>
					(m.agent_action?.type === 'message'
						? {
								type: 'message',
								content: m.content
							}
						: m.agent_action?.type === 'tool_call'
							? {
									type: 'tool_call',
									job_id: m.agent_action.job_id,
									module_id: m.agent_action.module_id,
									function_name: m.agent_action.function_name
								}
							: m.agent_action?.type === 'mcp_tool_call'
								? {
										type: 'mcp_tool_call',
										content: m.content,
										call_id: m.agent_action.call_id,
										function_name: m.agent_action.function_name,
										arguments: m.agent_action.arguments
									}
								: undefined) as AgentActionWithContent | undefined
			)
			.filter((m) => m !== undefined)

		await loadMissingJobs(agentActions)

		job = {
			...agentJob,
			raw_flow: {
				modules: agentActions
					.map((toolCall, idx) => {
						if (toolCall.type === 'message') {
							return {
								id: idx.toString(),
								value: {
									type: 'identity' as const
								}
							}
						} else if (toolCall.type === 'mcp_tool_call') {
							return {
								id: idx.toString(),
								value: {
									type: 'identity' as const
								},
								summary: toolCall.function_name,
								arguments: toolCall.arguments
							}
						} else {
							const module = tools.find((m) => m.summary === toolCall.function_name)
							return module
								? ({
										...module,
										id: idx.toString()
									} as FlowModule)
								: undefined
						}
					})
					.filter((m) => m !== undefined)
			}
		}
	}

	onMount(() => {
		loadToolCalls()
	})
</script>

{#if job}
	<div class="p-2">
		<FlowLogViewerWrapper
			{job}
			localModuleStates={fakeModuleStates}
			{workspaceId}
			render={true}
			onSelectedIteration={async () => {}}
			mode="aiagent"
		/>
	</div>
{/if}
