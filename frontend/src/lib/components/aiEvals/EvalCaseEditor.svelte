<script lang="ts">
	import { Button } from '$lib/components/common'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Label from '$lib/components/Label.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { Play, Save, X } from 'lucide-svelte'
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

	// The fields are edited as local state seeded from the draft, not bound through it: the parent
	// replaces `draft` wholesale to switch case and remounts this component with it, so seeding
	// once is exactly right and the inputs never have to track a draft changing underneath them.
	let userMessage = $state(draft.input?.user_message ?? '')
	// A conversation is edited as raw JSON in a plain textarea: it is captured from real traffic far
	// more often than it is typed, so a turn-by-turn editor would be a lot of surface for a rare
	// hand edit.
	let messagesText = $state(
		draft.input?.messages ? JSON.stringify(draft.input.messages, null, 2) : ''
	)
	let showConversation = $state((draft.input?.messages?.length ?? 0) > 0)
	// A captured answer is a string; a structured one is shown as JSON. Text that parses as JSON is
	// stored as JSON, so an expected value can be authored in either shape rather than only captured.
	let expectedText = $state(
		draft.expected == undefined
			? ''
			: typeof draft.expected === 'string'
				? draft.expected
				: JSON.stringify(draft.expected, null, 2)
	)
	let tagInput = $state('')

	function addTag() {
		const tag = tagInput.trim()
		if (!tag) return
		if (!(draft.tags ?? []).includes(tag)) {
			draft.tags = [...(draft.tags ?? []), tag]
		}
		tagInput = ''
	}

	let attachments = $derived((draft.input?.user_attachments ?? []) as { s3?: string }[])

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
		const text = expectedText.trim()
		let next: unknown = undefined
		if (text) {
			try {
				next = JSON.parse(text)
			} catch {
				next = expectedText
			}
		}
		if (!deepEqual(draft.expected, next)) {
			draft.expected = next
		}
	})

	$effect(() => {
		if (parsed.error) return
		const next = { ...draft.input, user_message: userMessage, messages: parsed.messages }
		if (!deepEqual(draft.input, next)) {
			draft.input = next
		}
	})
</script>

<div class="flex flex-col gap-4">
	<Label label="Name">
		<TextInput
			bind:value={draft.name}
			size="sm"
			inputProps={{ placeholder: 'Optional label for this case' }}
		/>
	</Label>

	<Label label="User message">
		<TextInput
			underlyingInputEl="textarea"
			size="sm"
			unifiedHeight={false}
			class="min-h-24"
			bind:value={userMessage}
			inputProps={{ placeholder: 'What the agent is asked' }}
		/>
	</Label>

	{#if attachments.length > 0}
		<Label label="Attachments">
			<div class="flex flex-wrap gap-1">
				{#each attachments as attachment, index (index)}
					<Badge color="gray">{attachment.s3 ?? JSON.stringify(attachment)}</Badge>
				{/each}
			</div>
		</Label>
	{/if}

	<Label
		label="Expected"
		tooltip="What a correct answer looks like. Every scorer is handed it alongside the answer the run produced; leave it empty for scorers that judge an answer on its own. Plain text, or JSON for a structured answer."
	>
		<TextInput
			underlyingInputEl="textarea"
			size="sm"
			unifiedHeight={false}
			class="min-h-16"
			bind:value={expectedText}
			inputProps={{ placeholder: 'The answer this case should produce (optional)' }}
		/>
	</Label>

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
			<TextInput
				underlyingInputEl="textarea"
				size="sm"
				unifiedHeight={false}
				class="min-h-40 font-mono !text-2xs"
				bind:value={messagesText}
				error={parsed.error}
				inputProps={{ placeholder: '[{ "role": "user", "content": "..." }]', spellcheck: false }}
			/>
			{#if parsed.error}
				<span class="text-red-500 text-2xs font-normal">{parsed.error}</span>
			{/if}
		{/if}
	</div>

	<Label
		label="Host flow"
		tooltip="A linked agent's tools bind to the flow they run in. Leave this empty to use the agent's own authored defaults, or name a flow to reproduce that flow's tool inputs instead."
	>
		<TextInput
			bind:value={draft.host_flow_path}
			size="sm"
			inputProps={{ placeholder: 'f/folder/flow (optional)' }}
		/>
	</Label>

	<Label label="Tags" tooltip="Free-form labels, kept with the case so a dataset can be grouped.">
		<div class="flex flex-col gap-1">
			{#if draft.tags?.length}
				<div class="flex flex-wrap gap-1">
					{#each draft.tags as tag, index (tag + index)}
						<Badge color="gray">
							{tag}
							<button
								class="rounded-full p-0.5 text-tertiary hover:bg-surface-hover hover:text-primary"
								title="Remove tag"
								aria-label="Remove tag"
								onclick={() => (draft.tags = draft.tags?.filter((_, i) => i !== index))}
							>
								<X size={11} />
							</button>
						</Badge>
					{/each}
				</div>
			{/if}
			<TextInput
				bind:value={tagInput}
				size="sm"
				inputProps={{
					placeholder: 'Add a tag',
					onkeydown: (e: KeyboardEvent) => e.key === 'Enter' && addTag()
				}}
			/>
		</div>
	</Label>

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
