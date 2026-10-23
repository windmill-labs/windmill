<script lang="ts">
	import { SecondsInput } from '$lib/components/common'
	import Label from '$lib/components/Label.svelte'
	import FlowCard from '../common/FlowCard.svelte'
	import { BUILTIN_TOOLS, type AgentTool } from '../agentToolUtils'

	/** Matches `DEFAULT_TIMEOUT_SECS` in `windmill-worker/src/ai/pause.rs`. */
	const DEFAULT_QUESTION_TIMEOUT_SECS = 1800

	let { tool = $bindable(), noEditor = false }: { tool: AgentTool; noEditor?: boolean } = $props()

	let value = $derived(tool.value as { tool_type: 'question'; timeout?: number })
</script>

<FlowCard
	{noEditor}
	title={BUILTIN_TOOLS.question.label}
	subtitle="The agent waits for the answer from the run page or the chat, without holding a worker."
	subtitleDocLink="https://www.windmill.dev/docs/core_concepts/ai_agents"
>
	<div class="flex flex-col gap-4 overflow-auto p-4" style="scrollbar-gutter: stable">
		<Label label="Timeout">
			<p class="text-xs font-normal text-secondary">Past it, the agent is told nobody answered</p>
			<SecondsInput
				disabled={noEditor}
				bind:seconds={
					() => value.timeout ?? DEFAULT_QUESTION_TIMEOUT_SECS, (v) => (value.timeout = v)
				}
			/>
		</Label>
	</div>
</FlowCard>
