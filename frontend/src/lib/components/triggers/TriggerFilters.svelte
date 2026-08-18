<script lang="ts">
	import Section from '$lib/components/Section.svelte'
	import TriggerFilterList from './TriggerFilterList.svelte'
	import type { FilterLogic, FilterNode, GroupOp } from './filters'

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

	// Only groups can negate, so the list never hands the root a 'none' back.
	function setRootLogic(op: GroupOp) {
		if (op !== 'none') filterLogic = op
	}

	let description = $derived(
		filterLogic === 'or'
			? 'Filters will limit the execution of the trigger to only messages that match any criterion.'
			: 'Filters will limit the execution of the trigger to only messages that match all criteria.'
	)

	let filterHelp = $derived(
		'Each criterion checks that the field is equal to, or a superset of, the filter value. ' +
			'A Key names a top-level field of the message (parsed as JSON); a Path reaches a nested one, e.g. data.status. Paths do not traverse arrays — match those with an array value instead. ' +
			'Add a group to nest criteria under their own logic, e.g. an OR of two fields inside an AND, or a NONE group to exclude messages that match it.' +
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
		<TriggerFilterList bind:filters bind:logic={() => filterLogic, setRootLogic} {disabled} />
	</div>
</Section>
