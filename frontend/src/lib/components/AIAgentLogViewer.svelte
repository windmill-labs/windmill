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
	import { untrack } from 'svelte'
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
						}),
						z.object({
							type: z.literal('web_search')
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
		noPadding?: boolean
	}

	let {
		tools,
		agentJob,
		workspaceId,
		onToolJobLoaded,
		storedToolCallJobs,
		noPadding = false
	}: Props = $props()

	const fakeModuleStates: Record<string, GraphModuleState> = $state({})

	async function loadMissingJobs(
		agentActions: AgentActionWithContent[],
		gen: number
	): Promise<Record<string, GraphModuleState>> {
		const states: Record<string, GraphModuleState> = {}
		const promises = agentActions.map(async (toolCall, idx) => {
			if (toolCall.type === 'tool_call') {
				let job: Job | undefined = storedToolCallJobs?.[idx]

				if (!job || job.type !== 'CompletedJob') {
					job = await JobService.getJob({
						id: toolCall.job_id,
						workspace: workspaceId ?? $workspaceStore!
					})
				}
				states[idx.toString()] = {
					args: job.args,
					type: job['success'] ? 'Success' : 'Failure',
					logs: job.logs,
					result: job['result'],
					job_id: toolCall.job_id
				}
				// Keyed by index in the parent's cache, so a superseded run must not write into it.
				if (gen === loadGen) {
					onToolJobLoaded?.(job, idx)
				}
			} else if (toolCall.type === 'mcp_tool_call') {
				states[idx.toString()] = {
					type: 'Success',
					args: toolCall.arguments ?? {},
					logs: '',
					result: toolCall.content
				}
			} else if (toolCall.type === 'web_search') {
				states[idx.toString()] = {
					type: 'Success',
					args: {},
					logs: '',
					result: toolCall.content
				}
			} else {
				states[idx.toString()] = {
					type: 'Success',
					args: {},
					logs: '',
					result: toolCall.content
				}
			}
		})

		await Promise.all(promises)
		return states
	}

	let job: Partial<Job> | undefined = $state(undefined)
	// Every prop change starts another load; only the newest may write the shared view, else a
	// slower reload for a previous run restores its logs over the one now selected.
	let loadGen = 0
	async function loadToolCalls(agentJob: Props['agentJob'], tools: AgentTool[]) {
		const gen = ++loadGen
		let parsedResult = resultSchema.safeParse(agentJob.result)
		if (!parsedResult.success) {
			console.error('Invalid result', parsedResult.error)
			// A failed agent job has no parseable action list. Drop the view rather than leave the
			// previously selected step's tool tree rendered under this one's header.
			if (gen === loadGen) {
				job = undefined
				for (const key of Object.keys(fakeModuleStates)) {
					delete fakeModuleStates[key]
				}
			}
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
								: m.agent_action?.type === 'web_search'
									? {
											type: 'web_search',
											content: m.content
										}
									: undefined) as AgentActionWithContent | undefined
			)
			.filter((m) => m !== undefined)

		const states = await loadMissingJobs(agentActions, gen)
		if (gen !== loadGen) {
			return
		}
		for (const key of Object.keys(fakeModuleStates)) {
			delete fakeModuleStates[key]
		}
		Object.assign(fakeModuleStates, states)

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
						} else if (toolCall.type === 'web_search') {
							return {
								id: idx.toString(),
								value: {
									type: 'identity' as const
								},
								summary: 'Web Search'
							}
						} else {
							const module = tools.find((m) => m.summary === toolCall.function_name)
							// A definition can be missing for a call that did run: the tool was renamed or
							// removed since, or it belongs to a linked agent whose resource is no longer
							// readable. Keep the recorded call — its args, logs and result come from the
							// child job — rather than dropping it from the history.
							return module
								? ({
										...module,
										id: idx.toString()
									} as FlowModule)
								: ({
										id: idx.toString(),
										value: { type: 'identity' as const },
										summary: toolCall.function_name
									} as FlowModule)
						}
					})
			}
		}
	}

	// Identity, not a summary digest: a refreshed resource can change a tool's path, code or id while
	// keeping its name and count. The store swaps the array only when its contents actually differ,
	// so one version per array instance tracks that exactly. An empty list is always the same key,
	// since callers hand out a fresh [] for it on every render.
	const toolsVersions = new WeakMap<object, number>()
	let nextToolsVersion = 0
	function toolsIdentity(list: AgentTool[]): string {
		if (list.length === 0) {
			return 'empty'
		}
		let version = toolsVersions.get(list)
		if (version === undefined) {
			version = ++nextToolsVersion
			toolsVersions.set(list, version)
		}
		return String(version)
	}

	// Rebuild when the inputs change, not only on mount: a linked agent's tools resolve
	// asynchronously after the first render, and switching between completed runs reuses this
	// component — either would otherwise keep the first snapshot. Keyed by value, because callers
	// rebuild the `agentJob` object on every render and identity alone would reload in a loop.
	let reloadKey = $derived(`${agentJob?.id ?? ''}|${toolsIdentity(tools)}`)
	$effect(() => {
		reloadKey
		untrack(() => {
			if (agentJob) {
				loadToolCalls(agentJob, tools)
			}
		})
	})
</script>

{#if job}
	<div class={noPadding ? '' : 'p-2'}>
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
