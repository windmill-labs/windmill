<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import { Check } from 'lucide-svelte'
	import FlowChat from '$lib/components/flows/conversations/FlowChat.svelte'
	import {
		AGENT_CHAT_SCHEMA,
		AGENT_STEP_ID,
		agentChatPath
	} from '$lib/components/flows/conversations/agentEditorChat'
	import type { FlowModule } from '$lib/gen'

	/**
	 * Records the demo of a saved AI agent in the agent's own chat: a new conversation, one run
	 * per message. What is saved is read off those runs, and the Hub page draws it with the same
	 * chat components, so what the recorder sees here is what a visitor reads.
	 */
	interface Props {
		/** The saved agent, by path. */
		path: string
		/** How many messages were sent, answered or not. */
		sent: number
		saving?: boolean
		/** The agent keeps no managed memory, so it cannot hold a conversation. */
		forgets?: boolean
		onrun: (
			userMessage: string,
			conversationId: string,
			inputs?: Record<string, any>
		) => Promise<string | undefined>
		/** Absent until the bundle is on the Hub: there is nothing to attach a recording to. */
		onsave?: () => void
	}

	let { path, sent, saving = false, forgets = false, onrun, onsave }: Props = $props()

	// The chat reads the agent's model through a step linked to it, as a flow's chat does.
	const modules = $derived<FlowModule[]>([
		{ id: AGENT_STEP_ID, value: { type: 'aiagent', agent: path, tools: [], input_transforms: {} } }
	] as unknown as FlowModule[])
</script>

<div class="flex h-full min-h-0 flex-col gap-3">
	{#if forgets}
		<Alert type="warning" size="xs" title="This agent keeps no managed memory">
			Each message is answered without the ones before it, but the Hub replays the recording as one
			conversation. Record a single message, or turn on managed memory on the agent first.
		</Alert>
	{/if}
	<div class="flex min-h-0 flex-1 flex-col rounded-md border">
		<FlowChat
			onRunFlow={onrun}
			path={agentChatPath(path)}
			subject="agent"
			frame="none"
			hideSidebar
			inputSchema={AGENT_CHAT_SCHEMA}
			flowModules={modules}
		/>
	</div>

	<div class="flex items-center justify-between gap-3">
		<span class="text-xs text-secondary">
			{#if !onsave}
				Push the bundle to the Hub first to save a conversation.
			{:else if sent === 0}
				Ask the agent what a visitor would ask it. Two or three messages make a good demo.
			{:else}
				The answered messages are saved as the demo on the agent's Hub page.
			{/if}
		</span>
		<Button
			variant="accent"
			unifiedSize="md"
			startIcon={{ icon: Check }}
			loading={saving}
			disabled={!onsave || sent === 0 || saving}
			onclick={() => onsave?.()}
		>
			Save as recording
		</Button>
	</div>
</div>
