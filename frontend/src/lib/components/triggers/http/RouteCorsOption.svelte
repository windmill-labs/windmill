<script lang="ts">
	import Label from '$lib/components/Label.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import type { Snippet } from 'svelte'

	interface Props {
		allowed_origins: string[] | undefined
		disabled?: boolean
		testingBadge?: Snippet | undefined
	}

	let {
		allowed_origins = $bindable(),
		disabled = false,
		testingBadge = undefined
	}: Props = $props()

	// The text field is the editing surface, `allowed_origins` the saved value.
	// Keeping them separate lets a half-typed entry stay on screen while the
	// trigger config holds only what parses.
	let raw = $state(allowed_origins?.join(', ') ?? '')
	let restricted = $state(allowed_origins !== undefined)

	function parse(value: string): string[] {
		return value
			.split(',')
			.map((origin) => origin.trim())
			.filter((origin) => origin !== '')
	}

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

	let origins = $derived(parse(raw))
	let error = $derived(origins.map(originError).find((error) => error !== undefined))

	$effect(() => {
		allowed_origins = restricted && !error && origins.length > 0 ? origins : undefined
	})
</script>

<Label label="Restrict origins" for="allowed-origins-toggle" class="w-full">
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
			error={error !== undefined}
		/>
		<div class="text-2xs text-secondary">
			Separate origins with commas. Use * to allow any origin.
		</div>
		{#if error}
			<div class="text-2xs text-red-600 dark:text-red-400">{error}</div>
		{/if}
	{/if}
</Label>
