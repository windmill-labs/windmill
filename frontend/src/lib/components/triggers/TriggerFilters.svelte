<script lang="ts">
	import Section from '$lib/components/Section.svelte'
	import TriggerFilterList from './TriggerFilterList.svelte'
	import type { FilterLogic, FilterNode } from './filters'

	interface Props {
		filters: FilterNode[]
		filterLogic: FilterLogic
		disabled?: boolean
		// Set when the runnable receives the payload base64-encoded (e.g. Kafka).
		// Filters always run on the message parsed as JSON, so we clarify the distinction.
		payloadBase64Encoded?: boolean
	}

	let {
		filters = $bindable(),
		filterLogic = $bindable(),
		disabled = false,
		payloadBase64Encoded = false
	}: Props = $props()

	let description = $derived(
		filterLogic === 'or'
			? 'Filters will limit the execution of the trigger to only messages that match any criterion.'
			: 'Filters will limit the execution of the trigger to only messages that match all criteria.'
	)

	let filterHelp = $derived(
		'The JSON filter checks if the value at the key is equal or a superset of the filter value. ' +
			'Keys match top-level fields of the message (parsed as JSON); to match a nested field, set an object value (e.g. key data, value {"status": "active"}). ' +
			'Add a group to combine criteria with the opposite logic, e.g. an OR of two keys inside an AND.' +
			(payloadBase64Encoded
				? ' The runnable still receives the payload base64-encoded; filters run on the message before that encoding.'
				: '')
	)
</script>

<Section label="Filters">
	<p class="text-xs mb-1 text-primary">
		{description}<br />
		{filterHelp}
	</p>
	<div class="mt-1">
		<TriggerFilterList bind:filters bind:logic={filterLogic} {disabled} />
	</div>
</Section>
