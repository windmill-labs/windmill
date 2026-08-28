<script lang="ts">
	import Label from '$lib/components/Label.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { parseAllowedOrigins } from './utils'
	import type { Snippet } from 'svelte'

	interface Props {
		allowed_origins: string[] | undefined
		/**
		 * Why the list cannot be saved, owned and derived by the editor. Passed
		 * one way: this component displays it and never writes it back, so it
		 * cannot outlive the tab or go stale against the stored value.
		 */
		error?: string | undefined
		/** Fetched once by the editor, so the badge and this field agree. */
		instanceDefaultOrigins?: string[]
		disabled?: boolean
		testingBadge?: Snippet | undefined
	}

	let {
		allowed_origins = $bindable(),
		error = undefined,
		instanceDefaultOrigins = [],
		disabled = false,
		testingBadge = undefined
	}: Props = $props()

	// The text field is the editing surface, `allowed_origins` the saved value.
	// Keeping them separate lets a half-typed entry stay on screen while the
	// trigger config holds the parsed list.
	let raw = $state(allowed_origins?.join(', ') ?? '')
	let restricted = $state(allowed_origins !== undefined)

	// Independent of the toggle: it decides what the toggle is called, which must
	// not change as it is flipped.
	let hasInstanceDefault = $derived(
		instanceDefaultOrigins.length > 0 && !instanceDefaultOrigins.includes('*')
	)
	// Without this the toggle reading "off" would look like "callable from
	// anywhere" on an instance that has narrowed the default.
	let inheritsInstanceDefault = $derived(!restricted && hasInstanceDefault)

	let origins = $derived(parseAllowedOrigins(raw))

	// While the toggle is on, whatever is typed is what gets saved. A rejected
	// entry must never collapse to `undefined`, because `undefined` is stored as
	// NULL and NULL means "any origin", so a typo would lift the restriction
	// while the toggle still reads as on.
	$effect(() => {
		allowed_origins = restricted ? origins : undefined
	})

	// Re-seed the text field when the value is replaced from outside — applying
	// a draft, or resetting to deployed, both write the prop while this
	// component stays mounted. Comparing against what this component would
	// itself produce is what tells an external write apart from its own, so
	// typing is never clobbered mid-edit.
	$effect(() => {
		const incoming = allowed_origins
		const own = restricted ? origins : undefined
		if (JSON.stringify(incoming) !== JSON.stringify(own)) {
			raw = incoming?.join(', ') ?? ''
			restricted = incoming !== undefined
		}
	})
</script>

<!-- The label names what the toggle does here, the way the workspace-prefix
	 toggle relabels itself when an instance setting is in play: with a default
	 configured, turning this on overrides it rather than adding a restriction. -->
<Label
	label={hasInstanceDefault ? 'Override allowed origins' : 'Allowed origins'}
	for="allowed-origins-toggle"
	class="w-full"
>
	{#snippet header()}
		<Tooltip documentationLink="https://www.windmill.dev/docs/core_concepts/http_routing">
			Which origins may call this route from a browser. Other origins can still send the request,
			they just cannot read the response.
		</Tooltip>
		{#if testingBadge}
			{@render testingBadge()}
		{/if}
	{/snippet}
	{#snippet action()}
		<Toggle
			checked={restricted}
			on:change={() => {
				restricted = !restricted
			}}
			{disabled}
			id="allowed-origins-toggle"
		/>
	{/snippet}
	{#if restricted}
		<TextInput
			bind:value={raw}
			inputProps={{ autocomplete: 'off', disabled, placeholder: 'https://app.example.com' }}
			error={error !== undefined}
		/>
		<!-- One line, per the form guideline's single Input -> Validation/Hint
			 slot. An empty list is a real state rather than a mistake: it allows
			 no origin at all, so say that instead of demanding an entry. -->
		{#if error}
			<div class="text-2xs text-red-600 dark:text-red-400">{error}</div>
		{:else if origins.length === 0}
			<div class="text-2xs text-secondary">
				Allows no origin. Add one, comma-separated, or * to allow any.
			</div>
		{:else}
			<div class="text-2xs text-secondary">Comma-separated. Use * to allow any.</div>
		{/if}
	{:else if inheritsInstanceDefault}
		<div class="text-2xs text-secondary">
			Currently allows {instanceDefaultOrigins.join(', ')}, from the instance default.
		</div>
	{/if}
</Label>
