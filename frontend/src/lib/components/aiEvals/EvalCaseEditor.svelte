<script lang="ts">
	import { Button } from '$lib/components/common'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Label from '$lib/components/Label.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { Play } from 'lucide-svelte'
	import { deepEqual } from 'fast-equals'
	import { onDestroy, untrack } from 'svelte'
	import type { CaseDraft } from './evalCaseUtils'

	let {
		draft = $bindable(),
		running = false,
		canSave = true,
		onRun,
		onSave
	}: {
		draft: CaseDraft
		running?: boolean
		canSave?: boolean
		onRun: () => void
		onSave: () => void | Promise<void>
	} = $props()

	// The fields are edited as local state seeded from the draft, not bound through it: the parent
	// replaces `draft` wholesale to switch case and remounts this component with it, so seeding
	// once is exactly right and the inputs never have to track a draft changing underneath them.
	let userMessage = $state(draft.input?.user_message ?? '')
	// A captured answer is a string; a structured one is shown as JSON. Text that parses as JSON is
	// stored as JSON, so an expected value can be authored in either shape rather than only captured.
	let expectedText = $state(
		draft.expected == undefined
			? ''
			: typeof draft.expected === 'string'
				? draft.expected
				: JSON.stringify(draft.expected, null, 2)
	)
	let attachments = $derived((draft.input?.user_attachments ?? []) as { s3?: string }[])

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
		const next = { ...draft.input, user_message: userMessage }
		if (!deepEqual(draft.input, next)) {
			draft.input = next
		}
	})

	// A case is a row, and editing a row saves it. Debounced rather than saved per keystroke, and
	// with nothing to say about it: the row in the table is the case, so watching it follow what you
	// type is the confirmation, and a write that fails says so in a toast.
	let saveTimer: ReturnType<typeof setTimeout> | undefined = undefined
	let lastSaved = $state<string | undefined>(undefined)
	$effect(() => {
		const snapshot = JSON.stringify($state.snapshot(draft))
		const blocked = !canSave
		untrack(() => {
			if (lastSaved === undefined) {
				// The state the editor opened on is what is stored: saving it back would write the
				// case over itself on every case you merely look at.
				lastSaved = snapshot
				return
			}
			if (blocked || snapshot === lastSaved) return
			clearTimeout(saveTimer)
			saveTimer = setTimeout(() => {
				lastSaved = snapshot
				onSave()
			}, SAVE_DEBOUNCE_MS)
		})
	})
	onDestroy(() => clearTimeout(saveTimer))

	const SAVE_DEBOUNCE_MS = 600
</script>

<div class="flex flex-col gap-4">
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

	<div class="flex gap-2 justify-end items-center">
		<Button
			variant="default"
			size="xs"
			startIcon={{ icon: Play }}
			disabled={running || !draft.id}
			title={draft.id ? 'Run this case now' : 'Save the case first'}
			onclick={onRun}
		>
			{running ? 'Running' : 'Run'}
		</Button>
	</div>
</div>
