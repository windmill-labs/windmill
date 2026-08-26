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

	// Without this the toggle reading "off" would look like "callable from
	// anywhere" on an instance that has narrowed the default.
	let inheritsInstanceDefault = $derived(
		!restricted && instanceDefaultOrigins.length > 0 && !instanceDefaultOrigins.includes('*')
	)

	let origins = $derived(parse(raw))
	// Split from `error` so an entry that is merely still empty reads as a hint
	// rather than a rejection: both block the save, only one is a mistake.
	let malformed = $derived(
		restricted ? origins.map(originError).find((message) => message !== undefined) : undefined
	)

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

<!-- The label carries the inherited state, matching how the workspace-prefix
	 toggle reads "(enforced by instance setting)": an off toggle that is
	 nonetheless restricted has to say so on the control itself. -->
<Label
	label={inheritsInstanceDefault
		? 'Restrict origins (inherited from instance default)'
		: 'Restrict origins'}
	for="allowed-origins-toggle"
	class="w-full"
>
	{#snippet header()}
		<Tooltip documentationLink="https://www.windmill.dev/docs/core_concepts/http_routing">
			Lists the origins allowed to call this route from a browser. Windmill answers the preflight
			and the response with the origin that matches, so a page on any other origin cannot read the
			result. Requests from outside a browser are unaffected. Leave off to keep the route readable
			from any origin.
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
			 slot: the instructions give way to the message that replaces them. -->
		{#if malformed}
			<div class="text-2xs text-red-600 dark:text-red-400">{malformed}</div>
		{:else if error}
			<div class="text-2xs text-hint">{error}</div>
		{:else}
			<div class="text-2xs text-secondary">
				Separate origins with commas. Use * to allow any origin.
			</div>
		{/if}
	{:else if inheritsInstanceDefault}
		<div class="text-2xs text-secondary">
			Allows {instanceDefaultOrigins.join(', ')}. Turn this on to override the default for this
			route.
		</div>
	{/if}
</Label>
