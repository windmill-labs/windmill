<script lang="ts">
	import Label from '$lib/components/Label.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { parseAllowedOrigins } from './utils'
	import type { Snippet } from 'svelte'

	interface Props {
		allowed_origins: string[] | undefined
		/** Bound so the editor can block saving while the list is unusable. */
		error?: string | undefined
		/** Fetched once by the editor, so the badge and this field agree. */
		instanceDefaultOrigins?: string[]
		disabled?: boolean
		testingBadge?: Snippet | undefined
	}

	let {
		allowed_origins = $bindable(),
		error = $bindable(),
		instanceDefaultOrigins = [],
		disabled = false,
		testingBadge = undefined
	}: Props = $props()

	// The text field is the editing surface, `allowed_origins` the saved value.
	// Keeping them separate lets a half-typed entry stay on screen while the
	// trigger config holds the parsed list.
	let raw = $state(allowed_origins?.join(', ') ?? '')
	let restricted = $state(allowed_origins !== undefined)

	const parse = parseAllowedOrigins

	// Mirrors `validate_allowed_origins` in windmill-trigger-http so the error
	// shows before saving rather than as a 400 from the API.
	function originError(origin: string): string | undefined {
		if (origin === '*') return undefined
		// An Origin header is always visible ASCII, so this covers both embedded
		// whitespace and a non-punycoded IDN, which the backend rejects too.
		if (!/^[\x21-\x7e]+$/.test(origin))
			return `'${origin}' must contain only visible ASCII, with no whitespace`
		const [scheme, ...rest] = origin.split('://')
		if (rest.length !== 1) return `'${origin}' is missing a scheme, such as https://`
		const host = rest[0]
		if (host === '') return `'${origin}' is missing a host`
		if (host.includes('/')) return `'${origin}' must not contain a path or trailing slash`
		if (host.includes('?') || host.includes('#'))
			return `'${origin}' must not contain a query or fragment`
		if (host.includes('@')) return `'${origin}' must not contain userinfo`
		if (!/^[A-Za-z0-9.+-]+$/.test(scheme)) return `'${origin}' has an invalid scheme`
		return undefined
	}

	// Independent of the toggle: it decides what the toggle is called, which must
	// not change as it is flipped.
	let hasInstanceDefault = $derived(
		instanceDefaultOrigins.length > 0 && !instanceDefaultOrigins.includes('*')
	)
	// Without this the toggle reading "off" would look like "callable from
	// anywhere" on an instance that has narrowed the default.
	let inheritsInstanceDefault = $derived(!restricted && hasInstanceDefault)

	let origins = $derived(parse(raw))
	// Only a typed entry that cannot work is shown as an error. A list that is
	// merely still empty is where the reader has just arrived, not a mistake to
	// report back at them.
	let malformed = $derived(
		restricted ? origins.map(originError).find((message) => message !== undefined) : undefined
	)

	// Gates the save; only `malformed` is rendered. An empty list still blocks
	// saving, because storing it as NULL would mean "any origin".
	$effect(() => {
		error = restricted
			? (malformed ?? (origins.length === 0 ? 'Enter at least one origin' : undefined))
			: undefined
	})

	// While the toggle is on, whatever is typed is what gets saved — a rejected
	// entry must never collapse to `undefined`, because `undefined` is stored as
	// NULL and NULL means "any origin". A typo would otherwise lift the
	// restriction while the toggle still reads as on. `error` blocks the save,
	// and the backend rejects the same values if one ever gets past it.
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
			error={malformed !== undefined}
		/>
		<!-- One line, per the form guideline's single Input -> Validation/Hint
			 slot. The format stays on screen until something is actually wrong,
			 so it is there when the field first appears empty. -->
		{#if malformed}
			<div class="text-2xs text-red-600 dark:text-red-400">{malformed}</div>
		{:else}
			<div class="text-2xs text-secondary">
				At least one origin, comma-separated. Use * to allow any.
			</div>
		{/if}
	{:else if inheritsInstanceDefault}
		<div class="text-2xs text-secondary">
			Currently allows {instanceDefaultOrigins.join(', ')}, from the instance default.
		</div>
	{/if}
</Label>
