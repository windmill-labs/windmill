<script lang="ts">
	import Label from '$lib/components/Label.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'

	export let raw_string: boolean
	export let wrap_body: boolean
</script>

<Label label="Raw body" class="w-full">
	<svelte:fragment slot="header">
		<Tooltip
			documentationLink="https://www.windmill.dev/docs/core_concepts/http_routing#body-processing-options"
		>
			Provides the raw JSON payload as a string under the 'raw_string' key. Required for custom
			script authentication method and useful for signature verification or other advanced use
			cases.
		</Tooltip>
	</svelte:fragment>
	<svelte:fragment slot="action">
		<Toggle
			checked={raw_string}
			on:change={() => {
				raw_string = !raw_string
			}}
		/>
	</svelte:fragment>
</Label>
<Label label="Wrap body" class="w-full">
	<svelte:fragment slot="header">
		<Tooltip
			documentationLink="https://www.windmill.dev/docs/core_concepts/http_routing#body-processing-options"
			>Wraps the payload in an object under the 'body' key, useful for handling unknown payloads.</Tooltip
		>
	</svelte:fragment>
	<svelte:fragment slot="action">
		<Toggle
			checked={wrap_body}
			on:change={() => {
				wrap_body = !wrap_body
			}}
		/>
	</svelte:fragment>
</Label>
