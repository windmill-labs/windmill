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

	let disablable = $state(field && !(field?.type === 'static' && field?.value === false))
</script>

<Toggle
	bind:checked={disablable}
	size="xs"
	options={{
		right: 'Can be disabled'
	}}
	on:change={() => {
		if (disablable) {
			field = {
				type: 'evalv2',
				expr: 'false',
				fieldType: 'boolean',
				connections: []
			}
		} else {
			field = {
				type: 'static',
				value: false,
				fieldType: 'boolean'
			}
		}
	}}
/>

{#if disablable}
	<div transition:slide|local>
		<InputsSpecEditor
			key="tabDisabled {index}"
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
			customTitle={field?.['customTitle']}
			displayType={false}
		/>
	</div>
{/if}
