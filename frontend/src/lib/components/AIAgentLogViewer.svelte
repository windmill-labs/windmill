<script lang="ts">
	import type { GraphModuleState } from './graph'
	import { JobService, type FlowModule, type FlowStatusModule, type Job } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import FlowLogViewerWrapper from './FlowLogViewerWrapper.svelte'
	import { z } from 'zod'
	import { getToolCallId } from './graph/renderers/nodes/AIToolNode.svelte'

	type AgentActionWithContent = NonNullable<FlowStatusModule['agent_actions']>[number] & {
		content: string
	}

	const resultSchema = z.object({
		messages: z.array(
			z.object({
				role: z.string(),
				content: z.string().optional(),
				agent_action: z
					.union([
						z.object({
							type: z.literal('tool_call'),
							job_id: z.string(),
							module_id: z.string(),
							function_name: z.string()
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
		result: unknown
		tools: FlowModule[]
		agentJob: Partial<Job>
		localModuleStates?: Record<string, GraphModuleState>
		workspaceId?: string | undefined
	}

	let { result, tools, agentJob, localModuleStates, workspaceId }: Props = $props()

	async function loadModuleStates(agentActions: AgentActionWithContent[]) {
		const promises = agentActions.map(async (toolCall, idx) => {
			if (toolCall.type === 'tool_call') {
				const job = await JobService.getJob({
					id: toolCall.job_id,
					workspace: workspaceId ?? $workspaceStore!
				})
				localModuleStatesCopy.localModuleStates[getToolCallId(idx, toolCall.module_id)] = {
					args: job.args,
					type: job['success'] ? 'Success' : 'Failure',
					logs: job.logs,
					result: job['result'],
					job_id: toolCall.job_id
				}
			} else {
				localModuleStatesCopy.localModuleStates[getToolCallId(idx)] = {
					type: 'Success',
					args: {},
					logs: '',
					result: toolCall.content
				}
			}
		})

		await Promise.all(promises)
	}

	let localModuleStatesCopy: { localModuleStates: Record<string, GraphModuleState> } = $state.raw({
		localModuleStates: {}
	})
	let job: Partial<Job> | undefined = $state(undefined)
	async function loadToolCalls() {
		let parsedResult = resultSchema.safeParse(result)
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
							: undefined) as AgentActionWithContent | undefined
			)
			.filter((m) => m !== undefined)

		if (!localModuleStates) {
			await loadModuleStates(agentActions)
		} else {
			localModuleStatesCopy.localModuleStates = structuredClone($state.snapshot(localModuleStates))
			agentActions.forEach((action, idx) => {
				if (action.type === 'message') {
					localModuleStatesCopy.localModuleStates[getToolCallId(idx)] = {
						type: 'Success',
						args: {},
						logs: '',
						result: action.content
					}
				}
			})
		}

		job = {
			...agentJob,
			raw_flow: {
				modules: agentActions
					.map((toolCall, idx) => {
						if (toolCall.type === 'message') {
							return {
								id: getToolCallId(idx),
								value: {
									type: 'identity' as const
								}
							}
						} else {
							const module = tools.find((m) => m.summary === toolCall.function_name)
							return module
								? {
										...module,
										id: getToolCallId(idx, module.id)
									}
								: undefined
						}
					})
					.filter((m) => m !== undefined)
			}
		}
	}

	$effect(() => {
		loadToolCalls()
	})
</script>

{#if job}
	<div class="p-2">
		<FlowLogViewerWrapper
			{job}
			localModuleStates={localModuleStatesCopy.localModuleStates}
			{workspaceId}
			render={true}
			onSelectedIteration={async () => {}}
			mode="aiagent"
		/>
	</div>
{/if}
