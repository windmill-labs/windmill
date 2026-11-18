<script lang="ts">
	import { Button } from '$lib/components/common'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { Check, Loader2, Send, Wrench } from 'lucide-svelte'
	import type { AgentTurn } from '$lib/components/recording/types'

	/**
	 * Records the demo of a saved AI agent: a conversation, one run of the agent per message,
	 * in a conversation of its own. What is saved is what the Hub page replays, so the
	 * recorder sees the answers and the tools called before keeping it.
	 */
	interface Props {
		turns: AgentTurn[]
		/** The message being answered, while a run is in flight. */
		pending?: string
		error?: string
		saving?: boolean
		onsend: (message: string) => void
		/** Absent until the bundle is on the Hub: there is nothing to attach a recording to. */
		onsave?: () => void
	}

	let {
		turns,
		pending = undefined,
		error = undefined,
		saving = false,
		onsend,
		onsave
	}: Props = $props()

	let draft = $state('')

	function send() {
		const message = draft.trim()
		if (!message || pending !== undefined) return
		draft = ''
		onsend(message)
	}

	const fmt = (ms: number) => (ms < 1000 ? `${Math.round(ms)} ms` : `${(ms / 1000).toFixed(1)} s`)
</script>

<div class="flex h-full min-h-0 flex-col gap-3">
	<div class="min-h-0 flex-1 space-y-3 overflow-y-auto rounded-md border bg-surface-secondary p-3">
		{#if turns.length === 0 && pending === undefined}
			<span class="text-xs text-hint">
				Ask the agent what a visitor would ask it. Two or three messages that show what it does make
				a good demo.
			</span>
		{/if}
		{#each turns as turn, i (i)}
			<div class="flex justify-end">
				<div
					class="max-w-[85%] whitespace-pre-wrap rounded-md bg-surface-accent-selected px-3 py-2 text-sm"
				>
					{turn.user_message}
				</div>
			</div>
			{#if turn.tool_calls.length > 0}
				<div class="flex flex-wrap items-center gap-2 text-2xs text-secondary">
					{#each turn.tool_calls as call, j (j)}
						<span class="inline-flex items-center gap-1 font-mono">
							<Wrench size={10} />{call.name}
						</span>
					{/each}
				</div>
			{/if}
			<div class="flex justify-start">
				<div class="max-w-[85%] whitespace-pre-wrap rounded-md border bg-surface px-3 py-2 text-sm">
					{turn.answer}
				</div>
			</div>
			<div class="text-2xs text-hint">{fmt(turn.duration_ms)}</div>
		{/each}
		{#if pending !== undefined}
			<div class="flex justify-end">
				<div
					class="max-w-[85%] whitespace-pre-wrap rounded-md bg-surface-accent-selected px-3 py-2 text-sm"
				>
					{pending}
				</div>
			</div>
			<div class="flex items-center gap-1.5 text-xs text-secondary">
				<Loader2 size={12} class="animate-spin" /> The agent is answering…
			</div>
		{/if}
		{#if error}
			<div class="rounded-md border border-red-300 bg-red-50 p-2 text-xs text-red-700">
				{error}
			</div>
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
					disabled: pending !== undefined,
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
			disabled={pending !== undefined || !draft.trim()}
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
			startIcon={{ icon: saving ? Loader2 : Check }}
			disabled={!onsave || turns.length === 0 || pending !== undefined || saving}
			onclick={() => onsave?.()}
		>
			Save as recording
		</Button>
	</div>
</div>
