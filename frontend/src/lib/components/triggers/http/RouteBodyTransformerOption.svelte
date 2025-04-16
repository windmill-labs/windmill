<script lang="ts">
	import Label from '$lib/components/Label.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'

	interface Props {
		raw_string: boolean
		wrap_body: boolean
		disabled?: boolean
	}

	let { raw_string = $bindable(), wrap_body = $bindable(), disabled = false }: Props = $props()
</script>

<Label label="Raw body" class="w-full">
	{#snippet header()}
		<Tooltip>
			Provides the raw JSON payload as a string under the 'raw_string' key. Required for custom
			script authentication method and useful for signature verification or other advanced use
			cases.
		</Tooltip>
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
			>Wraps the payload in an object under the 'body' key, useful for handling unknown payloads.</Tooltip
		>
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
