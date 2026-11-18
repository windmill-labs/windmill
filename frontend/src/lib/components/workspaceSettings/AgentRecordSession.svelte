<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { Check, Send } from 'lucide-svelte'
	import type { AgentTurn } from '$lib/components/recording/types'
	import AgentRecordingTranscript from '$lib/components/recording/AgentRecordingTranscript.svelte'

	/**
	 * Records the demo of a saved AI agent: a conversation, one run of the agent per message,
	 * in a conversation of its own. What is saved is what the Hub page replays, so the
	 * recorder sees the answers and the tools called before keeping it.
	 */
	interface Props {
		turns: AgentTurn[]
		workspace: string
		/** The message being answered, while a run is in flight. */
		pending?: string
		error?: string
		saving?: boolean
		/** Resolves to whether the message was answered; it is given back to retype otherwise. */
		onsend: (message: string) => Promise<boolean>
		/** Absent until the bundle is on the Hub: there is nothing to attach a recording to. */
		onsave?: () => void
	}

	let {
		turns,
		workspace,
		pending = undefined,
		error = undefined,
		saving = false,
		onsend,
		onsave
	}: Props = $props()

	let draft = $state('')

	async function send() {
		const message = draft.trim()
		if (!message || pending !== undefined || saving) return
		draft = ''
		const answered = await onsend(message)
		// A turn that failed is not in the conversation: hand the message back.
		if (!answered && !draft) draft = message
	}
</script>

<div class="flex h-full min-h-0 flex-col gap-3">
	<div class="min-h-0 flex-1 space-y-3 overflow-y-auto rounded-md border bg-surface-secondary p-3">
		{#if turns.length === 0 && pending === undefined}
			<span class="text-xs text-hint">
				Ask the agent what a visitor would ask it. Two or three messages that show what it does make
				a good demo.
			</span>
		{/if}
		<AgentRecordingTranscript {turns} {pending} {workspace} />
		{#if error}
			<Alert type="error" size="xs" title="The agent did not answer">{error}</Alert>
		{/if}
	</div>

	<div class="flex items-end gap-2">
		<div class="flex-1">
			<TextInput
				underlyingInputEl="textarea"
				bind:value={draft}
				unifiedHeight={false}
				class="min-h-[60px]"
				inputProps={{
					placeholder: 'Message the agent',
					disabled: pending !== undefined || saving,
					onkeydown: (e) => {
						if (e.key === 'Enter' && !e.shiftKey) {
							e.preventDefault()
							send()
						}
					}
				}}
			/>
		</div>
		<Button
			variant="default"
			startIcon={{ icon: Send }}
			disabled={pending !== undefined || saving || !draft.trim()}
			onclick={send}
		>
			Send
		</Button>
	</div>

	<div class="flex items-center justify-between gap-3">
		<span class="text-xs text-secondary">
			{#if !onsave}
				Push the bundle to the Hub first to save a conversation.
			{:else if turns.length === 0}
				Nothing to save yet.
			{:else}
				{turns.length} message{turns.length === 1 ? '' : 's'} — saved as the demo on the agent's Hub
				page.
			{/if}
		</span>
		<Button
			variant="accent"
			startIcon={{ icon: Check }}
			loading={saving}
			disabled={!onsave || turns.length === 0 || pending !== undefined || saving}
			onclick={() => onsave?.()}
		>
			Save as recording
		</Button>
	</div>
</div>
