<script lang="ts">
	import { Button } from '$lib/components/common'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { Play, Save } from 'lucide-svelte'
	import { deepEqual } from 'fast-equals'
	import type { CaseDraft } from './evalCaseUtils'

	let {
		draft = $bindable(),
		running = false,
		canSave = true,
		saveLabel = 'Save to dataset',
		onRun,
		onSave
	}: {
		draft: CaseDraft
		running?: boolean
		canSave?: boolean
		saveLabel?: string
		onRun: () => void
		onSave: () => void
	} = $props()

	// A conversation is edited as raw JSON: it is captured from real traffic far more often than
	// it is typed, and a turn-by-turn editor would be a lot of surface for a rare hand edit.
	let messagesText = $state(
		draft.input?.messages ? JSON.stringify(draft.input.messages, null, 2) : ''
	)
	let showConversation = $state((draft.input?.messages?.length ?? 0) > 0)

	let parsed = $derived.by(() => {
		if (!showConversation || !messagesText.trim()) {
			return { messages: undefined, error: '' }
		}
		try {
			const value = JSON.parse(messagesText)
			return Array.isArray(value)
				? { messages: value, error: '' }
				: { messages: undefined, error: 'Prior turns must be a JSON array of messages' }
		} catch (e) {
			return { messages: undefined, error: String(e) }
		}
	})

	$effect(() => {
		if (parsed.error) return
		if (!deepEqual(draft.input?.messages, parsed.messages)) {
			draft.input = { ...draft.input, messages: parsed.messages }
		}
	})
</script>

<div class="flex flex-col gap-4">
	<label class="flex flex-col gap-1">
		<span class="text-xs font-semibold text-secondary">Name</span>
		<TextInput
			bind:value={draft.name}
			size="sm"
			inputProps={{ placeholder: 'Optional label for this case' }}
		/>
	</label>

	<label class="flex flex-col gap-1">
		<span class="text-xs font-semibold text-secondary">User message</span>
		<TextInput
			underlyingInputEl="textarea"
			size="sm"
			unifiedHeight={false}
			class="min-h-24"
			value={draft.input?.user_message ?? ''}
			inputProps={{
				placeholder: 'What the agent is asked',
				oninput: (e) => (draft.input = { ...draft.input, user_message: e.currentTarget.value })
			}}
		/>
	</label>

	<div class="flex flex-col gap-1">
		<Toggle
			bind:checked={showConversation}
			size="xs"
			options={{ right: 'Replay a prior conversation' }}
		/>
		{#if showConversation}
			<div class="text-xs text-tertiary mb-1">
				Passed as the agent's whole memory for this run
				<Tooltip>
					The turns below are sent as an explicit message list, so the agent reads and writes none
					of its stored memory. A production conversation using the same agent keeps its own memory
					untouched.
				</Tooltip>
			</div>
			<div class="h-40 border rounded-md overflow-hidden">
				<SimpleEditor lang="json" bind:code={messagesText} fixedOverflowWidgets={false} />
			</div>
			{#if parsed.error}
				<span class="text-xs text-red-600">{parsed.error}</span>
			{/if}
		{/if}
	</div>

	<label class="flex flex-col gap-1">
		<span class="text-xs font-semibold text-secondary">
			Host flow
			<Tooltip>
				A linked agent's tools bind to the flow they run in. Leave this empty to use the agent's own
				authored defaults, or name a flow to reproduce that flow's tool inputs instead.
			</Tooltip>
		</span>
		<TextInput
			bind:value={draft.host_flow_path}
			size="sm"
			inputProps={{ placeholder: 'f/folder/flow (optional)' }}
		/>
	</label>

	<div class="flex gap-2 justify-end">
		<Button
			variant="default"
			size="xs"
			startIcon={{ icon: Save }}
			disabled={!canSave || !!parsed.error}
			onclick={onSave}
		>
			{saveLabel}
		</Button>
		<Button
			variant="accent"
			size="xs"
			startIcon={{ icon: Play }}
			disabled={running || !!parsed.error}
			onclick={onRun}
		>
			{running ? 'Running' : 'Run'}
		</Button>
	</div>
</div>
