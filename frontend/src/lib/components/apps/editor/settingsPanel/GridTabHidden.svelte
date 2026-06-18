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

	let hideable = $state(field && !(field?.type === 'static' && field?.value === false))
</script>

<Toggle
	bind:checked={hideable}
	size="xs"
	options={{
		right: 'Can be hidden'
	}}
	on:change={() => {
		if (hideable) {
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

{#if hideable}
	<div transition:slide|local>
		<InputsSpecEditor
			key="tabHidden {index}"
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