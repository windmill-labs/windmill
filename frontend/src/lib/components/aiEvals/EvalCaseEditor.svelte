<script lang="ts">
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Label from '$lib/components/Label.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { deepEqual } from 'fast-equals'
	import type { CaseDraft } from './evalCaseUtils'

	let {
		draft = $bindable()
	}: {
		draft: CaseDraft
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
</div>
