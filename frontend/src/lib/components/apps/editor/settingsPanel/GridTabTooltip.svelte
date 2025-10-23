<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import type { RichConfiguration } from '../../types'
	import InputsSpecEditor from './InputsSpecEditor.svelte'
	import { slide } from 'svelte/transition'

	interface Props {
		id: string
		field: RichConfiguration
		index: number
	}

	let { id, field = $bindable(), index }: Props = $props()

	// Tooltip is active if it's not empty static value
	let hasTooltip = $state(
		field && !(field?.type === 'static' && (field?.value === '' || field?.value == null))
	)
</script>

<Toggle
	bind:checked={hasTooltip}
	size="xs"
	options={{
		right: 'Tooltip'
	}}
	on:change={() => {
		if (hasTooltip) {
			field = {
				type: 'evalv2',
				expr: "''",
				fieldType: 'text',
				connections: []
			}
		} else {
			field = {
				type: 'static',
				value: '',
				fieldType: 'text'
			}
		}
	}}
/>

{#if hasTooltip}
	<div transition:slide|local>
		<InputsSpecEditor
			key="tabTooltip {index}"
			bind:componentInput={field}
			{id}
			userInputEnabled={false}
			shouldCapitalize={true}
			resourceOnly={false}
			fieldType={field?.['fieldType']}
			subFieldType={field?.['subFieldType']}
			format={field?.['format']}
			selectOptions={field?.['selectOptions']}
			tooltip={field?.['tooltip']}
			fileUpload={field?.['fileUpload']}
			placeholder={field?.['placeholder']}
			customTitle="Tooltip"
			displayType={false}
		/>
	</div>
{/if}
