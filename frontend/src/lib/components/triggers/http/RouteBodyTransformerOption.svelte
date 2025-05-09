<script lang="ts">
	import Label from '$lib/components/Label.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import type { Snippet } from 'svelte'

	interface Props {
		raw_string: boolean
		wrap_body: boolean
		disabled?: boolean
		testingBadge?: Snippet | undefined
	}

	let {
		raw_string = $bindable(),
		wrap_body = $bindable(),
		disabled = false,
		testingBadge = undefined
	}: Props = $props()
</script>

<Label label="Raw body" class="w-full">
	{#snippet header()}
		<Tooltip
			documentationLink="https://www.windmill.dev/docs/core_concepts/http_routing#body-processing-options"
		>
			Provides the raw JSON payload as a string under the 'raw_string' key. Required for custom
			script authentication method and useful for signature verification or other advanced use
			cases.
		</Tooltip>
		{#if testingBadge}
			{@render testingBadge()}
		{/if}
	{/snippet}
	{#snippet action()}
		<Toggle
			checked={raw_string}
			on:change={() => {
				raw_string = !raw_string
			}}
			{disabled}
		/>
	{/snippet}
</Label>
<Label label="Wrap body" class="w-full">
	{#snippet header()}
		<Tooltip
			documentationLink="https://www.windmill.dev/docs/core_concepts/http_routing#body-processing-options"
			>Wraps the payload in an object under the 'body' key, useful for handling unknown payloads.</Tooltip
		>
		{#if testingBadge}
			{@render testingBadge()}
		{/if}
	{/snippet}
	{#snippet action()}
		<Toggle
			checked={wrap_body}
			on:change={() => {
				wrap_body = !wrap_body
			}}
			{disabled}
		/>
	{/snippet}
</Label>
