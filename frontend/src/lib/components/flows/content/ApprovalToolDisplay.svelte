<script lang="ts">
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import { SecondsInput } from '$lib/components/common'
	import Label from '$lib/components/Label.svelte'
	import FlowCard from '../common/FlowCard.svelte'
	import { BUILTIN_TOOLS, type AgentTool } from '../agentToolUtils'

	/** Matches `DEFAULT_APPROVAL_TIMEOUT_SECS` in `windmill-worker/src/ai/pause.rs`. */
	const DEFAULT_APPROVAL_TIMEOUT_SECS = 1800

	let { tool = $bindable(), noEditor = false }: { tool: AgentTool; noEditor?: boolean } = $props()

	let value = $derived(tool.value as { tool_type: 'approval'; timeout?: number })
</script>

<FlowCard {noEditor} title={BUILTIN_TOOLS.approval.label}>
	<div class="flex flex-col gap-6 overflow-auto p-4" style="scrollbar-gutter: stable">
		<Alert title="The agent asks before it acts" type="info">
			The agent can call <code>request_approval</code> with a message. The run stops without holding
			a worker until someone approves or rejects it from the run page, then the agent continues with
			the answer and any comment.
		</Alert>
		<Label
			label="Timeout"
			tooltip="How long to wait for an answer. Past it, the agent is told nobody answered."
		>
			<SecondsInput
				disabled={noEditor}
				bind:seconds={() => value.timeout ?? DEFAULT_APPROVAL_TIMEOUT_SECS, (v) => (value.timeout = v)}
			/>
		</Label>
	</div>
</FlowCard>
